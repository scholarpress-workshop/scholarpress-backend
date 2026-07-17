#![allow(non_snake_case)]
use crate::document::*;
use roxmltree::Node;
use std::io::Read;

#[derive(Debug, Clone, Default)]
struct StyleInfo {
    font_name: Option<String>,
    font_size_half_pt: Option<f32>,
    bold: bool,
    italic: bool,
    underline: bool,
}

pub fn extract_docx(bytes: &[u8]) -> Result<ParsedDocument, Box<dyn std::error::Error>> {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    let doc_xml = {
        let mut file = archive.by_name("word/document.xml")?;
        let mut xml = String::new();
        file.read_to_string(&mut xml)?;
        xml
    };

    let styles = archive.by_name("word/styles.xml").ok().and_then(|mut f| {
        let mut s = String::new();
        f.read_to_string(&mut s).ok()?;
        Some(s)
    });

    let style_map = styles.as_deref().map(parse_styles).unwrap_or_default();
    let doc_tree = roxmltree::Document::parse(&doc_xml)?;
    let paragraphs = parse_paragraphs(doc_tree.root(), &style_map);

    tracing::info!(para_count = paragraphs.len(), "parsed docx paragraphs");
    if let Some(first) = paragraphs.first() {
        tracing::info!(first_text = %first.text.chars().take(80).collect::<String>());
    }

    if paragraphs.is_empty() {
        return Err("No text found in document. The file may be empty or use an unsupported XML namespace.".into());
    }

    let word_count: usize = paragraphs
        .iter()
        .map(|p| p.text.split_whitespace().count())
        .sum();
    let estimated_pages = (word_count as f32 / 250.0).ceil() as usize;

    let mut fonts = std::collections::BTreeSet::new();
    for p in &paragraphs {
        if let Some(ref name) = p.font_name {
            fonts.insert(name.clone());
        }
    }

    let raw_text: String = paragraphs
        .iter()
        .map(|p| p.text.clone())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(ParsedDocument {
        raw_text: raw_text.clone(),
        pages: vec![ParsedPage {
            page_number: 1,
            text: raw_text.clone(),
            width: 612.0,
            height: 792.0,
            spans: Vec::new(),
            images: Vec::new(),
            paths: Vec::new(),
        }],
        paragraphs,
        headings: Vec::new(),
        metadata: ParsedMetadata {
            title: None,
            author: None,
            page_count: estimated_pages.max(1),
            page_count_estimated: true,
            detected_fonts: fonts.into_iter().collect(),
        },
    })
}

fn parse_styles(xml: &str) -> std::collections::HashMap<String, StyleInfo> {
    let mut map = std::collections::HashMap::new();
    let doc = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(_) => return map,
    };

    let ns = resolve_ns(doc.root());
    let w_ns = ns.get("w").cloned().unwrap_or_default();

    for style_node in doc
        .descendants()
        .filter(|n| n.has_tag_name((w_ns.as_str(), "style")))
    {
        let style_id = style_node
            .attribute((w_ns.as_str(), "styleId"))
            .unwrap_or("")
            .to_string();
        if style_id.is_empty() {
            continue;
        }

        let mut info = StyleInfo::default();

        if let Some(rpr) = find_child(&style_node, &w_ns, "rPr") {
            if let Some(rfonts) = find_child(&rpr, &w_ns, "rFonts") {
                info.font_name = rfonts.attribute((w_ns.as_str(), "ascii")).map(String::from);
            }
            if let Some(sz) = find_child(&rpr, &w_ns, "sz") {
                if let Ok(val) = sz
                    .attribute((w_ns.as_str(), "val"))
                    .unwrap_or("0")
                    .parse::<f32>()
                {
                    info.font_size_half_pt = Some(val);
                }
            }
            info.bold = find_child(&rpr, &w_ns, "b").is_some();
            info.italic = find_child(&rpr, &w_ns, "i").is_some();
            info.underline = find_child(&rpr, &w_ns, "u").is_some();
        }

        map.insert(style_id, info);
    }

    map
}

fn parse_paragraphs(
    root: Node,
    style_map: &std::collections::HashMap<String, StyleInfo>,
) -> Vec<ParsedParagraph> {
    let mut paragraphs = Vec::new();
    let ns = resolve_ns(root);
    let w_ns = ns.get("w").cloned().unwrap_or_default();

    let body = find_child(&root, &w_ns, "body");
    let container = body.as_ref().unwrap_or(&root);

    for p_node in container
        .descendants()
        .filter(|n| n.has_tag_name((w_ns.as_str(), "p")))
    {
        let mut text = String::new();
        let mut bold = false;
        let mut italic = false;
        let mut underline = false;
        let mut font_size_half_pt = None;
        let mut font_name: Option<String> = None;

        if let Some(pPr) = find_child(&p_node, &w_ns, "pPr") {
            if let Some(pStyle) = find_child(&pPr, &w_ns, "pStyle") {
                if let Some(style_id) = pStyle.attribute((w_ns.as_str(), "val")) {
                    if let Some(style_info) = style_map.get(style_id) {
                        bold = bold || style_info.bold;
                        italic = italic || style_info.italic;
                        underline = underline || style_info.underline;
                        if font_size_half_pt.is_none() {
                            font_size_half_pt = style_info.font_size_half_pt;
                        }
                        if font_name.is_none() {
                            font_name = style_info.font_name.clone();
                        }
                    }
                }
            }
        }

        for r_node in p_node
            .descendants()
            .filter(|n| n.has_tag_name((w_ns.as_str(), "r")))
        {
            let mut run_bold = bold;
            let mut run_italic = italic;
            let mut run_underline = underline;
            let mut run_font_size = font_size_half_pt;
            let mut run_font_name = font_name.clone();

            if let Some(rPr) = find_child(&r_node, &w_ns, "rPr") {
                run_bold = find_child(&rPr, &w_ns, "b").is_some() || run_bold;
                run_italic = find_child(&rPr, &w_ns, "i").is_some() || run_italic;
                run_underline = find_child(&rPr, &w_ns, "u").is_some() || run_underline;

                if let Some(sz) = find_child(&rPr, &w_ns, "sz") {
                    if let Ok(val) = sz
                        .attribute((w_ns.as_str(), "val"))
                        .unwrap_or("0")
                        .parse::<f32>()
                    {
                        run_font_size = Some(val);
                    }
                }
                if let Some(rfonts) = find_child(&rPr, &w_ns, "rFonts") {
                    if let Some(ascii) = rfonts.attribute((w_ns.as_str(), "ascii")) {
                        run_font_name = Some(ascii.to_string());
                    }
                }
            }

            for t_node in r_node
                .descendants()
                .filter(|n| n.has_tag_name((w_ns.as_str(), "t")))
            {
                if let Some(t) = t_node.text() {
                    text.push_str(t);
                }
            }

            bold = run_bold;
            italic = run_italic;
            underline = run_underline;
            font_size_half_pt = run_font_size;
            font_name = run_font_name.clone();
        }

        if !text.trim().is_empty() {
            let all_caps = text
                .trim()
                .chars()
                .all(|c| !c.is_alphabetic() || c.is_uppercase());
            paragraphs.push(ParsedParagraph {
                text: text.trim().to_string(),
                page_number: 1,
                is_bold: bold,
                is_italic: italic,
                is_underline: underline,
                is_all_caps: all_caps,
                is_heading: false,
                heading_level: None,
                font_size: font_size_half_pt.map(|s| s / 2.0),
                font_name,
            });
        }
    }

    paragraphs
}

fn resolve_ns(root: Node) -> std::collections::HashMap<String, String> {
    let mut ns = std::collections::HashMap::new();
    for n in root.namespaces() {
        if n.uri() == "http://schemas.openxmlformats.org/wordprocessingml/2006/main" {
            ns.insert("w".to_string(), n.uri().to_string());
        }
    }
    ns
}

fn find_child<'a>(node: &Node<'a, 'a>, ns: &str, local: &str) -> Option<Node<'a, 'a>> {
    node.children().find(|n| {
        n.is_element() && n.tag_name().namespace() == Some(ns) && n.tag_name().name() == local
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_docx_invalid() {
        let result = extract_docx(b"not a zip file");
        assert!(result.is_err());
    }
}
