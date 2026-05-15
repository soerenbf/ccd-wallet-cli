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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FuzzySelectItem<T> {
    pub value: T,
    pub text: String,
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

    let ordered = order_items(items, initial);
    let mut picker = select(prompt);
    for item in ordered {
        picker = picker.item(item.value, item.label, item.hint);
    }
    Ok(picker.interact()?)
}

pub(crate) fn fuzzy_select_or_single<T>(prompt: &str, items: &[FuzzySelectItem<T>]) -> Result<T>
where
    T: Clone + Eq,
{
    if items.len() == 1 {
        return Ok(items[0].value.clone());
    }

    let mut picker = select(prompt).filter_mode().max_rows(10);
    for item in items {
        picker = picker.item(item.value.clone(), item.text.clone(), "");
    }
    Ok(picker.interact()?)
}

fn order_items<T>(items: &[SelectItem<T>], initial: Option<&T>) -> Vec<SelectItem<T>>
where
    T: Clone + Eq,
{
    let Some(initial) = initial else {
        return items.to_vec();
    };

    let mut ordered = items.to_vec();
    if let Some(index) = ordered.iter().position(|item| item.value == *initial) {
        let initial_item = ordered.remove(index);
        ordered.insert(0, initial_item);
    }
    ordered
}

#[cfg(test)]
mod tests {
    use super::{
        ContextLine, ResolutionSource, SelectItem, format_resolved_context, order_items,
        select_or_single,
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

    #[test]
    fn order_items_moves_initial_to_front() {
        let ordered = order_items(
            &[
                SelectItem {
                    value: "mainnet".to_owned(),
                    label: "mainnet".to_owned(),
                    hint: String::new(),
                },
                SelectItem {
                    value: "testnet".to_owned(),
                    label: "testnet".to_owned(),
                    hint: String::new(),
                },
            ],
            Some(&"testnet".to_owned()),
        );

        assert_eq!(ordered[0].value, "testnet");
        assert_eq!(ordered[1].value, "mainnet");
    }
}
