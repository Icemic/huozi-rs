mod color_space;
mod glyph_span;
mod layout_output;
mod layout_style;
pub(crate) mod tiqian_input;
pub(crate) mod tiqian_output;
mod vertex;

use std::collections::HashMap;

use anyhow::Result;

use crate::Huozi;
use crate::glyph_vertices::UnitVertices;
use crate::parser::{
    ParsedParagraph, ParsedText, Segment, SegmentId, SourceRange, TextRun, TextSpan, TextStyle,
    lower_elements, parse, parse_with,
};

use self::tiqian_input::{HuoziTiqianInput, HuoziTiqianInputAdapter};
use self::tiqian_output::HuoziTiqianOutputAdapter;

pub use self::color_space::*;
pub use self::glyph_span::*;
pub use self::layout_output::*;
pub use self::layout_style::*;
pub use self::vertex::*;

impl Huozi {
    /// Parse the text into text spans.
    pub fn parse_text(
        &self,
        segments: &Vec<Segment>,
        initial_text_style: &TextStyle,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<ParsedText, String> {
        let elements = segments
            .iter()
            .map(|segment| parse(segment))
            .collect::<Result<Vec<Vec<_>>, String>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(lower_elements(elements, initial_text_style, style_prefabs))
    }

    /// Parse the text with custom open and close tag characters.
    pub fn parse_text_with<const OPEN: char, const CLOSE: char>(
        &self,
        segments: &Vec<Segment>,
        initial_text_style: &TextStyle,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<ParsedText, String> {
        let elements = segments
            .iter()
            .map(|segment| parse_with::<OPEN, CLOSE>(segment))
            .collect::<Result<Vec<Vec<_>>, String>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(lower_elements(elements, initial_text_style, style_prefabs))
    }

    /// Parse the text into text spans, then layout it into glyph vertices.
    pub fn layout_parse(
        &mut self,
        segments: &Vec<Segment>,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<RichTextLayoutOutput, String> {
        let text = self.parse_text(segments, initial_text_style, style_prefabs)?;
        Ok(self.layout_parsed_text(layout_style, &text, initial_text_style, color_space))
    }

    /// Parse text with custom tag symbols, then layout it into glyph vertices.
    pub fn layout_parse_with<const OPEN: char, const CLOSE: char>(
        &mut self,
        segments: &Vec<Segment>,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<RichTextLayoutOutput, String> {
        let text =
            self.parse_text_with::<OPEN, CLOSE>(segments, initial_text_style, style_prefabs)?;
        Ok(self.layout_parsed_text(layout_style, &text, initial_text_style, color_space))
    }

    /// Layout source segments without interpreting rich-text tags.
    pub fn layout_plain(
        &mut self,
        segments: &Vec<Segment>,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
    ) -> Result<RichTextLayoutOutput, String> {
        let text_spans = segments
            .iter()
            .map(|segment| TextSpan {
                span_id: None,
                runs: vec![TextRun {
                    text: segment.content.to_string(),
                    style: initial_text_style.clone(),
                    source_range: SourceRange {
                        segment_id: segment.id.clone(),
                        start: Default::default(),
                        end: crate::parser::ScalarOffset(segment.content.chars().count()),
                    },
                }],
            })
            .collect::<Vec<_>>();
        Ok(self.layout(layout_style, &text_spans, color_space))
    }

    /// Layout text spans through tiqian's shaping and paragraph layout pipeline.
    pub fn layout<T: AsRef<Vec<TextSpan>>>(
        &mut self,
        layout_style: &LayoutStyle,
        text_spans: T,
        color_space: ColorSpace,
    ) -> RichTextLayoutOutput {
        let initial_text_style = text_spans
            .as_ref()
            .first()
            .and_then(|span| span.runs.first())
            .map(|run| run.style.clone())
            .unwrap_or_default();
        let input =
            HuoziTiqianInputAdapter::adapt(text_spans.as_ref(), layout_style, &initial_text_style);
        let mut document = DocumentBuilder::new();
        let remaining = document.remaining_height(layout_style);
        document.append_input(self, input, &color_space, remaining);
        document.finish()
    }

    fn layout_parsed_text(
        &mut self,
        layout_style: &LayoutStyle,
        text: &ParsedText,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
    ) -> RichTextLayoutOutput {
        let mut document = DocumentBuilder::new();
        for paragraph in &text.paragraphs {
            // 空段占一行高度：它没有行内内容，Tiqian 对空输入返回零行。
            if paragraph.nodes.is_empty() {
                document.push_empty_paragraph(layout_style, initial_text_style, paragraph);
                continue;
            }
            let Some(remaining) = document.remaining_height(layout_style) else {
                break;
            };
            let input = HuoziTiqianInputAdapter::adapt_paragraph(
                paragraph,
                layout_style,
                initial_text_style,
                remaining,
                document.next_continuity(),
            );
            document.append_input(self, input, &color_space, Some(remaining));
        }
        document.finish()
    }
}

/// 把多段 Tiqian 结果组合成一个文档级公开结果。
///
/// 每段独立走 Tiqian；段落自身的坐标加上前面段落的累积高度与视觉行数后进入文档坐标。
struct DocumentBuilder {
    glyphs: Vec<UnitVertices>,
    glyph_segments: Vec<Option<SegmentId>>,
    interactions: Vec<Interaction>,
    width: f32,
    /// 已输出段落的累积高度。
    height: f32,
    /// 已输出段落的累积视觉行数。
    rows: u32,
    continuity: u32,
}

impl DocumentBuilder {
    fn new() -> Self {
        Self {
            glyphs: Vec::new(),
            glyph_segments: Vec::new(),
            interactions: Vec::new(),
            width: 0.0,
            height: 0.0,
            rows: 0,
            continuity: 0,
        }
    }

    fn next_continuity(&self) -> u32 {
        self.continuity
    }

    /// 返回该段可用的剩余高度。
    fn remaining_height(&self, layout_style: &LayoutStyle) -> Option<f32> {
        match layout_style.box_height {
            Some(box_height) => {
                let remaining = box_height as f32 - self.height;
                (remaining > 0.0).then_some(remaining)
            }
            None => Some(f32::INFINITY),
        }
    }

    /// 追加一个空段：只占一行高度并推进视觉行号。
    fn push_empty_paragraph(
        &mut self,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        paragraph: &ParsedParagraph,
    ) {
        let line_height = paragraph.paragraph_style.line_height.map_or(
            (initial_text_style.font_size * layout_style.line_height) as f32,
            |line_height| line_height,
        );
        let Some(remaining) = self.remaining_height(layout_style) else {
            return;
        };
        if line_height > remaining {
            return;
        }
        self.height += line_height;
        self.rows += 1;
    }

    /// 布局一个已适配的段落并追加其结果。
    fn append_input(
        &mut self,
        huozi: &mut Huozi,
        input: HuoziTiqianInput,
        color_space: &ColorSpace,
        remaining_height: Option<f32>,
    ) {
        let Some(remaining) = remaining_height else {
            return;
        };
        let continuity = input.continuity;
        let result = huozi.layout_engine.layout(input.layout_input);
        let output = HuoziTiqianOutputAdapter::adapt(
            huozi,
            &result,
            &input.source_map,
            color_space,
            self.rows,
            self.height,
            remaining,
        );
        self.continuity = self.continuity.max(continuity);
        if output.visual_lines == 0 {
            return;
        }
        self.width = self.width.max(output.width);
        self.height += output.height;
        self.rows += output.visual_lines;
        self.glyphs.extend(output.glyphs);
        self.glyph_segments.extend(output.glyph_segments);
        self.interactions.extend(output.interactions);
    }

    fn finish(mut self) -> RichTextLayoutOutput {
        let segment_glyph_spans = self::tiqian_output::segment_glyph_spans(&self.glyph_segments);
        self.glyph_segments.clear();
        RichTextLayoutOutput {
            glyphs: self.glyphs,
            segment_glyph_spans,
            interactions: self.interactions,
            width: self.width.max(0.0).round() as u32,
            height: self.height.max(0.0).round() as u32,
        }
    }
}
