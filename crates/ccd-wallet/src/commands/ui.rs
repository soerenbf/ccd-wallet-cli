use anyhow::Result;
use cliclack::select;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResolutionSource {
    Prompted,
    ActiveDefault,
    Explicit,
    Inferred,
}

impl ResolutionSource {
    pub(crate) fn should_display(self) -> bool {
        !matches!(self, Self::Prompted)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContextLine<'a> {
    pub label: &'a str,
    pub value: String,
    pub source: ResolutionSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectItem<T> {
    pub value: T,
    pub label: String,
    pub hint: String,
}

pub(crate) fn format_resolved_context(lines: &[ContextLine<'_>]) -> Option<String> {
    let visible = lines
        .iter()
        .filter(|line| line.source.should_display())
        .collect::<Vec<_>>();
    if visible.is_empty() {
        return None;
    }

    let width = visible
        .iter()
        .map(|line| line.label.len())
        .max()
        .unwrap_or(0);
    Some(
        visible
            .into_iter()
            .map(|line| format!("{:<width$} {}", line.label, line.value, width = width))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub(crate) fn log_resolved_context(lines: &[ContextLine<'_>]) -> Result<()> {
    if let Some(message) = format_resolved_context(lines) {
        cliclack::log::info(message)?;
    }
    Ok(())
}

pub(crate) fn select_or_single<T>(
    prompt: &str,
    items: &[SelectItem<T>],
    initial: Option<&T>,
) -> Result<T>
where
    T: Clone + Eq,
{
    if items.len() == 1 {
        return Ok(items[0].value.clone());
    }

    let mut picker = select(prompt);
    for item in items {
        picker = picker.item(item.value.clone(), item.label.clone(), item.hint.clone());
    }
    if let Some(initial) =
        initial.filter(|initial| items.iter().any(|item| item.value == **initial))
    {
        picker = picker.initial_value(initial.clone());
    }
    Ok(picker.interact()?)
}

#[cfg(test)]
mod tests {
    use super::{
        ContextLine, ResolutionSource, SelectItem, format_resolved_context, select_or_single,
    };

    #[test]
    fn formats_visible_context_lines_with_alignment() {
        let message = format_resolved_context(&[
            ContextLine {
                label: "seed:",
                value: "test".to_owned(),
                source: ResolutionSource::ActiveDefault,
            },
            ContextLine {
                label: "network:",
                value: "testnet @ https://grpc.testnet.concordium.com:20000".to_owned(),
                source: ResolutionSource::Explicit,
            },
        ])
        .unwrap();

        assert_eq!(
            message,
            "seed:    test\nnetwork: testnet @ https://grpc.testnet.concordium.com:20000"
        );
    }

    #[test]
    fn hides_prompted_context_lines() {
        let message = format_resolved_context(&[
            ContextLine {
                label: "seed:",
                value: "test".to_owned(),
                source: ResolutionSource::Prompted,
            },
            ContextLine {
                label: "network:",
                value: "testnet @ https://grpc.testnet.concordium.com:20000".to_owned(),
                source: ResolutionSource::Prompted,
            },
        ]);

        assert_eq!(message, None);
    }

    #[test]
    fn select_or_single_skips_prompt_for_single_item() {
        let selected = select_or_single(
            "Select network",
            &[SelectItem {
                value: "testnet".to_owned(),
                label: "testnet".to_owned(),
                hint: "https://grpc.testnet.concordium.com:20000".to_owned(),
            }],
            None,
        )
        .unwrap();

        assert_eq!(selected, "testnet");
    }
}
