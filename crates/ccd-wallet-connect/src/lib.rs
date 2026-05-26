//! WebSocket JSON-RPC server for `ccd-wallet connect` browser sessions.
//!
//! Supported JSON-RPC methods:
//! - `pair`: approve a browser session and return a session token.
//! - `requestAccount`: return the network/account context bound during pairing.
//! - `requestContractInit`: request wallet approval for a smart contract init transaction.
//! - `requestContractUpdate`: request wallet approval for a smart contract update transaction.

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
};
use tracing::{debug, info};
use uuid::Uuid;

pub const PAIR_METHOD: &str = "pair";
pub const REQUEST_ACCOUNT_METHOD: &str = "requestAccount";
pub const REQUEST_CONTRACT_INIT_METHOD: &str = "requestContractInit";
pub const REQUEST_CONTRACT_UPDATE_METHOD: &str = "requestContractUpdate";
const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RequestAccountResult {
    #[serde(rename = "accountAddress")]
    pub account_address: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairingRequest {
    pub origin: String,
    pub challenge: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairingApproval {
    pub network_genesis_hash: String,
    pub account_address: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountRequest {
    pub network_genesis_hash: String,
    pub account_address: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountRequestApproval {
    pub account_address: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ContractAddress {
    pub index: u64,
    pub subindex: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContractInitRequest {
    pub origin: String,
    pub network_genesis_hash: String,
    pub account_address: String,
    pub module_ref: String,
    pub init_name: String,
    pub amount_micro_ccd: String,
    pub max_contract_execution_energy: u64,
    pub parameter_hex: String,
    pub schema: Option<Value>,
    pub validate: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContractUpdateRequest {
    pub origin: String,
    pub network_genesis_hash: String,
    pub account_address: String,
    pub contract_address: ContractAddress,
    pub receive_name: String,
    pub amount_micro_ccd: String,
    pub max_contract_execution_energy: u64,
    pub parameter_hex: String,
    pub schema: Option<Value>,
    pub validate: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractInitApproval {
    pub transaction_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractUpdateApproval {
    pub transaction_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractExecutionErrorKind {
    UserDeclined,
    SubmissionFailed,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractExecutionRejection {
    pub kind: ContractExecutionErrorKind,
    pub message: String,
}

impl ContractExecutionRejection {
    pub fn user_declined(message: impl Into<String>) -> Self {
        Self {
            kind: ContractExecutionErrorKind::UserDeclined,
            message: message.into(),
        }
    }

    pub fn submission_failed(message: impl Into<String>) -> Self {
        Self {
            kind: ContractExecutionErrorKind::SubmissionFailed,
            message: message.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        Self {
            kind: ContractExecutionErrorKind::Other,
            message: message.into(),
        }
    }

    fn json_rpc_code(&self) -> i64 {
        match self.kind {
            ContractExecutionErrorKind::UserDeclined => -32004,
            ContractExecutionErrorKind::SubmissionFailed => -32005,
            ContractExecutionErrorKind::Other => -32000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairingRejection {
    pub message: String,
}

impl PairingRejection {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

type PairingApprover = Arc<
    dyn Fn(
            PairingRequest,
        ) -> BoxFuture<'static, std::result::Result<PairingApproval, PairingRejection>>
        + Send
        + Sync,
>;

pub type ContractInitApprover = Arc<
    dyn Fn(
            ContractInitRequest,
        ) -> BoxFuture<
            'static,
            std::result::Result<ContractInitApproval, ContractExecutionRejection>,
        > + Send
        + Sync,
>;

pub type ContractUpdateApprover = Arc<
    dyn Fn(
            ContractUpdateRequest,
        ) -> BoxFuture<
            'static,
            std::result::Result<ContractUpdateApproval, ContractExecutionRejection>,
        > + Send
        + Sync,
>;

#[derive(Clone)]
pub struct ConnectServer {
    approver: PairingApprover,
    contract_init_approver: ContractInitApprover,
    contract_update_approver: ContractUpdateApprover,
    state: Arc<Mutex<ServerState>>,
}

impl ConnectServer {
    pub fn new<F, G, H, I>(
        approver: F,
        _account_requester: G,
        contract_init_approver: H,
        contract_update_approver: I,
    ) -> Self
    where
        F: Fn(
                PairingRequest,
            )
                -> BoxFuture<'static, std::result::Result<PairingApproval, PairingRejection>>
            + Send
            + Sync
            + 'static,
        G: Fn(
                AccountRequest,
            )
                -> BoxFuture<'static, std::result::Result<AccountRequestApproval, PairingRejection>>
            + Send
            + Sync
            + 'static,
        H: Fn(
                ContractInitRequest,
            ) -> BoxFuture<
                'static,
                std::result::Result<ContractInitApproval, ContractExecutionRejection>,
            > + Send
            + Sync
            + 'static,
        I: Fn(
                ContractUpdateRequest,
            ) -> BoxFuture<
                'static,
                std::result::Result<ContractUpdateApproval, ContractExecutionRejection>,
            > + Send
            + Sync
            + 'static,
    {
        Self {
            approver: Arc::new(approver),
            contract_init_approver: Arc::new(contract_init_approver),
            contract_update_approver: Arc::new(contract_update_approver),
            state: Arc::new(Mutex::new(ServerState::default())),
        }
    }

    pub async fn serve(self, addr: SocketAddr, shutdown: oneshot::Receiver<()>) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .with_context(|| format!("failed to bind connect server to {addr}"))?;
        self.serve_listener(listener, shutdown).await
    }

    pub async fn serve_listener(
        self,
        listener: TcpListener,
        mut shutdown: oneshot::Receiver<()>,
    ) -> Result<()> {
        let local_addr = listener
            .local_addr()
            .context("failed to read connect server local address")?;
        info!(%local_addr, "connect server listening");

        loop {
            tokio::select! {
                biased;
                _ = &mut shutdown => {
                    info!(%local_addr, "connect server shutting down");
                    return Ok(());
                }
                accepted = listener.accept() => {
                    let (stream, peer_addr) = accepted.context("failed to accept websocket connection")?;
                    debug!(%peer_addr, "accepted connect client");
                    let server = self.clone();
                    tokio::spawn(async move {
                        if let Err(err) = server.handle_tcp_stream(stream).await {
                            debug!(%peer_addr, error = %err, "connect client ended with error");
                        }
                    });
                }
            }
        }
    }

    async fn handle_tcp_stream(self, mut stream: TcpStream) -> Result<()> {
        let request = read_handshake_request(&mut stream).await?;
        let origin = request
            .headers
            .get("origin")
            .cloned()
            .context("websocket Origin header is required")?;
        validate_origin(&origin)?;
        let websocket_key = request
            .headers
            .get("sec-websocket-key")
            .context("Sec-WebSocket-Key header is required")?;
        write_handshake_response(&mut stream, websocket_key).await?;
        self.handle_websocket(stream, origin).await
    }

    async fn handle_websocket(self, mut stream: TcpStream, origin: String) -> Result<()> {
        loop {
            match read_frame(&mut stream).await? {
                ClientFrame::Text(text) => {
                    let response = self.handle_text_message(&origin, &text).await;
                    write_text_frame(&mut stream, &response).await?;
                }
                ClientFrame::Ping(payload) => {
                    write_control_frame(&mut stream, 0xA, &payload).await?
                }
                ClientFrame::Close => {
                    let _ = write_control_frame(&mut stream, 0x8, &[]).await;
                    return Ok(());
                }
                ClientFrame::Other => {}
            }
        }
    }

    async fn handle_text_message(&self, origin: &str, text: &str) -> String {
        let request = match serde_json::from_str::<JsonRpcRequest>(text) {
            Ok(request) => request,
            Err(err) => return json_rpc_error(None, -32700, format!("parse error: {err}")),
        };
        let id = request.id.clone();
        match request.jsonrpc.as_deref() {
            Some("2.0") => {}
            _ => return json_rpc_error(id, -32600, "invalid JSON-RPC version"),
        }

        match request.method.as_str() {
            PAIR_METHOD => self.handle_pair(id, origin, request.params).await,
            REQUEST_ACCOUNT_METHOD => self.handle_request_account(id, request.params).await,
            REQUEST_CONTRACT_INIT_METHOD => {
                self.handle_contract_init(id, origin, request.params).await
            }
            REQUEST_CONTRACT_UPDATE_METHOD => {
                self.handle_contract_update(id, origin, request.params)
                    .await
            }
            _ => json_rpc_error(id, -32601, "method not found"),
        }
    }

    async fn handle_pair(&self, id: Option<Value>, origin: &str, params: Option<Value>) -> String {
        if self.active_session().is_some() {
            return json_rpc_error(id, -32001, "a browser session is already active");
        }
        let params = match params.map(serde_json::from_value::<PairParams>).transpose() {
            Ok(Some(params)) => params,
            Ok(None) => return json_rpc_error(id, -32602, "pair params are required"),
            Err(err) => return json_rpc_error(id, -32602, format!("invalid pair params: {err}")),
        };
        if let Err(err) = validate_challenge(&params.challenge) {
            return json_rpc_error(id, -32602, err.to_string());
        }

        let request = PairingRequest {
            origin: origin.to_owned(),
            challenge: params.challenge,
        };
        match (self.approver)(request).await {
            Ok(approval) => {
                let token = Uuid::new_v4().to_string();
                let session = ActiveSession {
                    token: token.clone(),
                    network_genesis_hash: approval.network_genesis_hash,
                    account_address: approval.account_address,
                };
                if !self.install_session(session) {
                    return json_rpc_error(id, -32001, "a browser session is already active");
                }
                json_rpc_result(
                    id,
                    json!({
                        "sessionToken": token,
                    }),
                )
            }
            Err(rejection) => json_rpc_error(id, -32000, rejection.message),
        }
    }

    async fn handle_request_account(&self, id: Option<Value>, params: Option<Value>) -> String {
        let params = match params
            .map(serde_json::from_value::<RequestAccountParams>)
            .transpose()
        {
            Ok(Some(params)) => params,
            Ok(None) => return json_rpc_error(id, -32602, "requestAccount params are required"),
            Err(err) => {
                return json_rpc_error(id, -32602, format!("invalid requestAccount params: {err}"));
            }
        };
        match self.active_session() {
            Some(session) if session.token == params.session_token => {
                if params.network_genesis_hash != session.network_genesis_hash {
                    return json_rpc_error(
                        id,
                        -32000,
                        "requestAccount networkGenesisHash does not match the active session",
                    );
                }
                json_rpc_result(
                    id,
                    json!(RequestAccountResult {
                        account_address: session.account_address,
                    }),
                )
            }
            Some(_) => json_rpc_error(id, -32003, "invalid session token"),
            None => json_rpc_error(id, -32002, "no active browser session"),
        }
    }

    async fn handle_contract_init(
        &self,
        id: Option<Value>,
        origin: &str,
        params: Option<Value>,
    ) -> String {
        let params = match params
            .map(serde_json::from_value::<ContractInitParams>)
            .transpose()
        {
            Ok(Some(params)) => params,
            Ok(None) => {
                return json_rpc_error(id, -32602, "requestContractInit params are required");
            }
            Err(err) => {
                return json_rpc_error(
                    id,
                    -32602,
                    format!("invalid requestContractInit params: {err}"),
                );
            }
        };
        let Some(session) = self.active_session() else {
            return json_rpc_error(id, -32002, "no active browser session");
        };
        if session.token != params.session_token {
            return json_rpc_error(id, -32003, "invalid session token");
        }

        let request = ContractInitRequest {
            origin: origin.to_owned(),
            network_genesis_hash: session.network_genesis_hash,
            account_address: session.account_address,
            module_ref: params.module_ref,
            init_name: params.init_name,
            amount_micro_ccd: params.amount_micro_ccd,
            max_contract_execution_energy: params.max_contract_execution_energy,
            parameter_hex: params.parameter_hex,
            schema: params.schema,
            validate: params.validate.unwrap_or(false),
        };
        match (self.contract_init_approver)(request).await {
            Ok(approval) => {
                json_rpc_result(id, json!({ "transactionHash": approval.transaction_hash }))
            }
            Err(rejection) => json_rpc_error(id, rejection.json_rpc_code(), rejection.message),
        }
    }

    async fn handle_contract_update(
        &self,
        id: Option<Value>,
        origin: &str,
        params: Option<Value>,
    ) -> String {
        let params = match params
            .map(serde_json::from_value::<ContractUpdateParams>)
            .transpose()
        {
            Ok(Some(params)) => params,
            Ok(None) => {
                return json_rpc_error(id, -32602, "requestContractUpdate params are required");
            }
            Err(err) => {
                return json_rpc_error(
                    id,
                    -32602,
                    format!("invalid requestContractUpdate params: {err}"),
                );
            }
        };
        let Some(session) = self.active_session() else {
            return json_rpc_error(id, -32002, "no active browser session");
        };
        if session.token != params.session_token {
            return json_rpc_error(id, -32003, "invalid session token");
        }

        let request = ContractUpdateRequest {
            origin: origin.to_owned(),
            network_genesis_hash: session.network_genesis_hash,
            account_address: session.account_address,
            contract_address: params.contract_address,
            receive_name: params.receive_name,
            amount_micro_ccd: params.amount_micro_ccd,
            max_contract_execution_energy: params.max_contract_execution_energy,
            parameter_hex: params.parameter_hex,
            schema: params.schema,
            validate: params.validate.unwrap_or(false),
        };
        match (self.contract_update_approver)(request).await {
            Ok(approval) => {
                json_rpc_result(id, json!({ "transactionHash": approval.transaction_hash }))
            }
            Err(rejection) => json_rpc_error(id, rejection.json_rpc_code(), rejection.message),
        }
    }

    fn active_session(&self) -> Option<ActiveSession> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.active_session.clone())
    }

    fn install_session(&self, session: ActiveSession) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        if state.active_session.is_some() {
            return false;
        }
        state.active_session = Some(session);
        true
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveSession {
    token: String,
    network_genesis_hash: String,
    account_address: String,
}

#[derive(Default)]
struct ServerState {
    active_session: Option<ActiveSession>,
}

#[derive(Debug)]
struct HandshakeRequest {
    headers: BTreeMap<String, String>,
}

#[derive(Debug)]
enum ClientFrame {
    Text(String),
    Ping(Vec<u8>),
    Close,
    Other,
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: Option<String>,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct PairParams {
    challenge: String,
}

#[derive(Debug, Deserialize)]
struct RequestAccountParams {
    #[serde(rename = "sessionToken")]
    session_token: String,
    #[serde(rename = "networkGenesisHash")]
    network_genesis_hash: String,
}

#[derive(Debug, Deserialize)]
struct ContractInitParams {
    #[serde(rename = "sessionToken")]
    session_token: String,
    #[serde(rename = "moduleRef")]
    module_ref: String,
    #[serde(rename = "initName")]
    init_name: String,
    #[serde(rename = "amountMicroCcd")]
    amount_micro_ccd: String,
    #[serde(rename = "maxContractExecutionEnergy")]
    max_contract_execution_energy: u64,
    #[serde(rename = "parameterHex")]
    parameter_hex: String,
    schema: Option<Value>,
    validate: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ContractUpdateParams {
    #[serde(rename = "sessionToken")]
    session_token: String,
    #[serde(rename = "contractAddress")]
    contract_address: ContractAddress,
    #[serde(rename = "receiveName")]
    receive_name: String,
    #[serde(rename = "amountMicroCcd")]
    amount_micro_ccd: String,
    #[serde(rename = "maxContractExecutionEnergy")]
    max_contract_execution_energy: u64,
    #[serde(rename = "parameterHex")]
    parameter_hex: String,
    schema: Option<Value>,
    validate: Option<bool>,
}

async fn read_handshake_request(stream: &mut TcpStream) -> Result<HandshakeRequest> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        let read = stream
            .read(&mut buffer)
            .await
            .context("failed to read websocket handshake")?;
        if read == 0 {
            bail!("connection closed during websocket handshake");
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if bytes.len() > 16 * 1024 {
            bail!("websocket handshake is too large");
        }
    }

    let raw = std::str::from_utf8(&bytes).context("websocket handshake is not valid UTF-8")?;
    let mut lines = raw.split("\r\n");
    let request_line = lines.next().context("websocket request line is missing")?;
    if !request_line.starts_with("GET ") {
        bail!("websocket handshake must use GET");
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
    }
    Ok(HandshakeRequest { headers })
}

async fn write_handshake_response(stream: &mut TcpStream, websocket_key: &str) -> Result<()> {
    let accept = websocket_accept(websocket_key);
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    stream
        .write_all(response.as_bytes())
        .await
        .context("failed to write websocket handshake response")
}

async fn read_frame(stream: &mut TcpStream) -> Result<ClientFrame> {
    let mut header = [0u8; 2];
    stream
        .read_exact(&mut header)
        .await
        .context("failed to read websocket frame header")?;
    let opcode = header[0] & 0x0F;
    let masked = header[1] & 0x80 != 0;
    let mut payload_len = u64::from(header[1] & 0x7F);
    if payload_len == 126 {
        let mut bytes = [0u8; 2];
        stream.read_exact(&mut bytes).await?;
        payload_len = u64::from(u16::from_be_bytes(bytes));
    } else if payload_len == 127 {
        let mut bytes = [0u8; 8];
        stream.read_exact(&mut bytes).await?;
        payload_len = u64::from_be_bytes(bytes);
    }
    if payload_len > 1024 * 1024 {
        bail!("websocket frame payload is too large");
    }
    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask).await?;
    }
    let mut payload = vec![0u8; payload_len as usize];
    stream.read_exact(&mut payload).await?;
    if masked {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }

    match opcode {
        0x1 => Ok(ClientFrame::Text(
            String::from_utf8(payload).context("websocket text frame is not UTF-8")?,
        )),
        0x8 => Ok(ClientFrame::Close),
        0x9 => Ok(ClientFrame::Ping(payload)),
        _ => Ok(ClientFrame::Other),
    }
}

async fn write_text_frame(stream: &mut TcpStream, text: &str) -> Result<()> {
    write_frame(stream, 0x1, text.as_bytes()).await
}

async fn write_control_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<()> {
    write_frame(stream, opcode, payload).await
}

async fn write_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> Result<()> {
    let mut frame = Vec::with_capacity(payload.len() + 10);
    frame.push(0x80 | opcode);
    match payload.len() {
        0..=125 => frame.push(payload.len() as u8),
        126..=65535 => {
            frame.push(126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        _ => {
            frame.push(127);
            frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
    }
    frame.extend_from_slice(payload);
    stream
        .write_all(&frame)
        .await
        .context("failed to write websocket frame")
}

fn websocket_accept(websocket_key: &str) -> String {
    let mut data = websocket_key.as_bytes().to_vec();
    data.extend_from_slice(WEBSOCKET_GUID.as_bytes());
    BASE64.encode(sha1(&data))
}

pub fn validate_origin(origin: &str) -> Result<()> {
    if origin.trim() != origin || origin.is_empty() {
        bail!("websocket Origin header must be a non-empty normalized value");
    }
    if origin == "null" {
        bail!("websocket Origin header must not be null");
    }
    if !(origin.starts_with("http://") || origin.starts_with("https://")) {
        bail!("websocket Origin header must use http or https");
    }
    Ok(())
}

pub fn validate_challenge(challenge: &str) -> Result<()> {
    if challenge.len() != 6 || !challenge.bytes().all(|byte| byte.is_ascii_digit()) {
        bail!("pairing challenge must be exactly six ASCII digits");
    }
    Ok(())
}

fn json_rpc_result(id: Option<Value>, result: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "result": result,
    })
    .to_string()
}

fn json_rpc_error(id: Option<Value>, code: i64, message: impl Into<String>) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": {
            "code": code,
            "message": message.into(),
        },
    })
    .to_string()
}

pub fn method_descriptions() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (PAIR_METHOD, "request browser pairing"),
        (
            REQUEST_ACCOUNT_METHOD,
            "request the account address bound to the active browser session",
        ),
        (
            REQUEST_CONTRACT_INIT_METHOD,
            "request approval to initialize a smart contract instance",
        ),
        (
            REQUEST_CONTRACT_UPDATE_METHOD,
            "request approval to invoke a smart contract receive function",
        ),
    ])
}

fn sha1(input: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let bit_len = (input.len() as u64) * 8;
    let mut message = input.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks(64) {
        let mut w = [0u32; 80];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            let offset = i * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for (i, word) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut output = [0u8; 20];
    for (index, value) in [h0, h1, h2, h3, h4].into_iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::FutureExt;

    fn server() -> ConnectServer {
        ConnectServer::new(
            |_| {
                async move {
                    Ok(PairingApproval {
                        network_genesis_hash: "genesis".to_owned(),
                        account_address: "addr".to_owned(),
                    })
                }
                .boxed()
            },
            |_| {
                async move {
                    Ok(AccountRequestApproval {
                        account_address: "unused".to_owned(),
                    })
                }
                .boxed()
            },
            |_| {
                async move {
                    Ok(ContractInitApproval {
                        transaction_hash: "init-hash".to_owned(),
                    })
                }
                .boxed()
            },
            |_| {
                async move {
                    Ok(ContractUpdateApproval {
                        transaction_hash: "update-hash".to_owned(),
                    })
                }
                .boxed()
            },
        )
    }

    #[test]
    fn computes_websocket_accept_value() {
        assert_eq!(
            websocket_accept("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn validates_origin() {
        validate_origin("https://example.com").unwrap();
        validate_origin("http://localhost:8080").unwrap();
        assert!(validate_origin("").is_err());
        assert!(validate_origin("null").is_err());
        assert!(validate_origin("file://example").is_err());
    }

    #[test]
    fn validates_challenge() {
        validate_challenge("123456").unwrap();
        assert!(validate_challenge("12345").is_err());
        assert!(validate_challenge("abcdef").is_err());
    }

    #[tokio::test]
    async fn pairing_success_returns_token_only_and_request_account_uses_it() {
        let server = server();
        let response = server
            .handle_text_message(
                "https://example.com",
                r#"{"jsonrpc":"2.0","id":1,"method":"pair","params":{"challenge":"123456"}}"#,
            )
            .await;
        let value: Value = serde_json::from_str(&response).unwrap();
        let token = value["result"]["sessionToken"].as_str().unwrap();
        assert_eq!(value["result"].get("context"), None);

        let response = server
            .handle_text_message(
                "https://example.com",
                &json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": REQUEST_ACCOUNT_METHOD,
                    "params": {
                        "sessionToken": token,
                        "networkGenesisHash": "genesis"
                    }
                })
                .to_string(),
            )
            .await;
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["result"]["accountAddress"], "addr");
    }

    #[tokio::test]
    async fn request_account_rejects_mismatched_genesis_hash() {
        let server = server();
        let pair_response = server
            .handle_text_message(
                "https://example.com",
                r#"{"jsonrpc":"2.0","id":1,"method":"pair","params":{"challenge":"123456"}}"#,
            )
            .await;
        let value: Value = serde_json::from_str(&pair_response).unwrap();
        let token = value["result"]["sessionToken"].as_str().unwrap();

        let response = server
            .handle_text_message(
                "https://example.com",
                &json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": REQUEST_ACCOUNT_METHOD,
                    "params": {
                        "sessionToken": token,
                        "networkGenesisHash": "other-genesis"
                    }
                })
                .to_string(),
            )
            .await;
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["error"]["code"], -32000);
    }

    #[tokio::test]
    async fn contract_execution_dispatches_with_session_context() {
        let server = server();
        let pair_response = server
            .handle_text_message(
                "https://example.com",
                r#"{"jsonrpc":"2.0","id":1,"method":"pair","params":{"challenge":"123456"}}"#,
            )
            .await;
        let value: Value = serde_json::from_str(&pair_response).unwrap();
        let token = value["result"]["sessionToken"].as_str().unwrap();

        let init_response = server
            .handle_text_message(
                "https://example.com",
                &json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": REQUEST_CONTRACT_INIT_METHOD,
                    "params": {
                        "sessionToken": token,
                        "moduleRef": "00",
                        "initName": "init_contract",
                        "amountMicroCcd": "0",
                        "maxContractExecutionEnergy": 30000,
                        "parameterHex": "",
                        "validate": true
                    }
                })
                .to_string(),
            )
            .await;
        let value: Value = serde_json::from_str(&init_response).unwrap();
        assert_eq!(value["result"]["transactionHash"], "init-hash");

        let update_response = server
            .handle_text_message(
                "https://example.com",
                &json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": REQUEST_CONTRACT_UPDATE_METHOD,
                    "params": {
                        "sessionToken": token,
                        "contractAddress": { "index": 1, "subindex": 0 },
                        "receiveName": "contract.receive",
                        "amountMicroCcd": "0",
                        "maxContractExecutionEnergy": 30000,
                        "parameterHex": ""
                    }
                })
                .to_string(),
            )
            .await;
        let value: Value = serde_json::from_str(&update_response).unwrap();
        assert_eq!(value["result"]["transactionHash"], "update-hash");
    }

    #[tokio::test]
    async fn contract_execution_rejects_invalid_session_token_and_parse_errors() {
        let server = server();
        let response = server
            .handle_text_message(
                "https://example.com",
                &json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": REQUEST_CONTRACT_UPDATE_METHOD,
                    "params": {
                        "sessionToken": "missing",
                        "contractAddress": { "index": 1, "subindex": 0 },
                        "receiveName": "contract.receive",
                        "amountMicroCcd": "0",
                        "maxContractExecutionEnergy": 30000,
                        "parameterHex": ""
                    }
                })
                .to_string(),
            )
            .await;
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["error"]["code"], -32002);

        let _ = server
            .handle_text_message(
                "https://example.com",
                r#"{"jsonrpc":"2.0","id":2,"method":"pair","params":{"challenge":"123456"}}"#,
            )
            .await;
        let response = server
            .handle_text_message(
                "https://example.com",
                &json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": REQUEST_CONTRACT_UPDATE_METHOD,
                    "params": { "sessionToken": "wrong" }
                })
                .to_string(),
            )
            .await;
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["error"]["code"], -32602);

        let response = server
            .handle_text_message(
                "https://example.com",
                &json!({
                    "jsonrpc": "2.0",
                    "id": 4,
                    "method": REQUEST_CONTRACT_UPDATE_METHOD,
                    "params": {
                        "sessionToken": "wrong",
                        "contractAddress": { "index": 1, "subindex": 0 },
                        "receiveName": "contract.receive",
                        "amountMicroCcd": "0",
                        "maxContractExecutionEnergy": 30000,
                        "parameterHex": ""
                    }
                })
                .to_string(),
            )
            .await;
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["error"]["code"], -32003);
    }

    #[test]
    fn install_session_preserves_bound_context() {
        let server = server();
        assert!(server.install_session(ActiveSession {
            token: "token".to_owned(),
            network_genesis_hash: "genesis".to_owned(),
            account_address: "addr".to_owned(),
        }));
        assert_eq!(
            server.active_session(),
            Some(ActiveSession {
                token: "token".to_owned(),
                network_genesis_hash: "genesis".to_owned(),
                account_address: "addr".to_owned(),
            })
        );
    }

    #[tokio::test]
    async fn rejected_pairing_returns_error() {
        let server = ConnectServer::new(
            |_| async move { Err(PairingRejection::new("rejected by user")) }.boxed(),
            |_| {
                async move {
                    Ok(AccountRequestApproval {
                        account_address: "addr".to_owned(),
                    })
                }
                .boxed()
            },
            |_| async move { Err(ContractExecutionRejection::other("unused")) }.boxed(),
            |_| async move { Err(ContractExecutionRejection::other("unused")) }.boxed(),
        );
        let response = server
            .handle_text_message(
                "https://example.com",
                r#"{"jsonrpc":"2.0","id":1,"method":"pair","params":{"challenge":"123456"}}"#,
            )
            .await;
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["error"]["message"], "rejected by user");
    }

    #[tokio::test]
    async fn active_session_rejects_additional_pairing() {
        let server = server();
        let _ = server
            .handle_text_message(
                "https://example.com",
                r#"{"jsonrpc":"2.0","id":1,"method":"pair","params":{"challenge":"123456"}}"#,
            )
            .await;
        let response = server
            .handle_text_message(
                "https://other.example",
                r#"{"jsonrpc":"2.0","id":2,"method":"pair","params":{"challenge":"654321"}}"#,
            )
            .await;
        let value: Value = serde_json::from_str(&response).unwrap();
        assert_eq!(value["error"]["code"], -32001);
    }
}
