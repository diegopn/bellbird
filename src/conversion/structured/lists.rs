use crate::document::{Block, Inline};

#[derive(Clone)]
pub(super) struct DoclingListItem {
    pub(super) ordered: bool,
    pub(super) number: u64,
    pub(super) first_in_list: bool,
    pub(super) level: u8,
    pub(super) marker: Option<String>,
    pub(super) content: Vec<Inline>,
}

pub(super) fn convert_docling_list_items(
    items: &[DoclingListItem],
    cursor: &mut usize,
    level: u8,
    simplified: &mut bool,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    while let Some(item) = items.get(*cursor) {
        let item_level = item.level;
        if item_level < level {
            break;
        }
        if item_level > level {
            *simplified = true;
            blocks.extend(convert_docling_list_items(
                items, cursor, item_level, simplified,
            ));
            continue;
        }

        let ordered = item.ordered;
        let start_number = item.number;
        let mut list = Vec::new();
        let mut item_labels = Vec::new();
        while let Some(item) = items.get(*cursor) {
            if item.level != level
                || item.ordered != ordered
                || (!list.is_empty() && item.first_in_list)
            {
                break;
            }
            let mut content = vec![Block::Paragraph(item.content.clone())];
            item_labels.push(item.marker.clone());
            *cursor += 1;
            if items.get(*cursor).is_some_and(|next| next.level > level) {
                content.extend(convert_docling_list_items(
                    items,
                    cursor,
                    level.saturating_add(1),
                    simplified,
                ));
            }
            list.push(content);
        }
        blocks.push(Block::List {
            ordered,
            start: (ordered && start_number != 1).then_some(start_number),
            item_labels,
            items: list,
        });
    }
    blocks
}
