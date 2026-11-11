use crate::document::Document;
use crate::error::BellbirdError;

#[cfg(test)]
use crate::document::{Block, ResourceReference};

use super::plain_text::extract_explicit_metadata;

mod inline;
mod lists;
mod nodes;

use nodes::convert_docling_nodes;

#[cfg(test)]
pub(super) fn convert_docling_node(
    node: &docling::Node,
    resources: &mut Vec<ResourceReference>,
    simplified: &mut bool,
) -> Vec<Block> {
    nodes::convert_docling_node(node, resources, simplified)
}

pub(super) fn convert_structured_document(
    display_name: String,
    extension: &str,
    bytes: &[u8],
) -> Result<Document, BellbirdError> {
    let format = docling::InputFormat::from_extension(extension)
        .filter(|format| format.is_compiled_in())
        .ok_or_else(|| BellbirdError::UnsupportedFormat(extension.to_owned()))?;
    let source = docling::SourceDocument::from_bytes(display_name, format, bytes.to_vec());
    let converted = docling::DocumentConverter::new()
        .strict(true)
        .fetch_images(false)
        .convert(source)
        .map_err(|error| BellbirdError::Conversion(error.to_string()))?;

    Ok(convert_docling_document(&converted.document))
}

fn convert_docling_document(document: &docling::DoclingDocument) -> Document {
    let mut converted = Document::default();
    converted.blocks = convert_docling_nodes(
        &document.nodes,
        &mut converted.resources,
        &mut converted.has_simplified_content,
    );
    extract_explicit_metadata(&mut converted);
    converted
}
