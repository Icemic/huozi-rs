mod color_space;
mod glyph_span;
mod layout_style;
pub(crate) mod tiqian_input;
pub(crate) mod tiqian_output;
mod vertex;

use std::collections::HashMap;

use anyhow::Result;

use crate::Huozi;
use crate::glyph_vertices::GlyphVertices;
use crate::parser::{
    Segment, SourceRange, TextRun, TextSpan, TextStyle, parse, parse_with, to_spans,
};

use self::tiqian_input::HuoziTiqianInputAdapter;
use self::tiqian_output::HuoziTiqianOutputAdapter;

pub use self::color_space::*;
pub use self::glyph_span::*;
pub use self::layout_style::*;
pub use self::vertex::*;

impl Huozi {
    /// Parse the text into text spans.
    pub fn parse_text(
        &self,
        segments: &Vec<Segment>,
        initial_text_style: &TextStyle,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<Vec<TextSpan>, String> {
        let elements = segments
            .iter()
            .map(|segment| parse(segment))
            .collect::<Result<Vec<Vec<_>>, String>>()?
            .into_iter()
            .flatten()
            .collect();
        to_spans(elements, initial_text_style, style_prefabs)
    }

    /// Parse the text with custom open and close tag characters.
    pub fn parse_text_with<const OPEN: char, const CLOSE: char>(
        &self,
        segments: &Vec<Segment>,
        initial_text_style: &TextStyle,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<Vec<TextSpan>, String> {
        let elements = segments
            .iter()
            .map(|segment| parse_with::<OPEN, CLOSE>(segment))
            .collect::<Result<Vec<Vec<_>>, String>>()?
            .into_iter()
            .flatten()
            .collect();
        to_spans(elements, initial_text_style, style_prefabs)
    }

    /// Parse the text into text spans, then layout it into glyph vertices.
    pub fn layout_parse(
        &mut self,
        segments: &Vec<Segment>,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<(Vec<GlyphVertices>, Vec<SegmentGlyphSpan>, u32, u32), String> {
        let text_spans = self.parse_text(segments, initial_text_style, style_prefabs)?;
        Ok(self.layout(layout_style, &text_spans, color_space))
    }

    /// Parse text with custom tag symbols, then layout it into glyph vertices.
    pub fn layout_parse_with<const OPEN: char, const CLOSE: char>(
        &mut self,
        segments: &Vec<Segment>,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
        style_prefabs: Option<&HashMap<String, TextStyle>>,
    ) -> Result<(Vec<GlyphVertices>, Vec<SegmentGlyphSpan>, u32, u32), String> {
        let text_spans =
            self.parse_text_with::<OPEN, CLOSE>(segments, initial_text_style, style_prefabs)?;
        Ok(self.layout(layout_style, &text_spans, color_space))
    }

    /// Layout source segments without interpreting rich-text tags.
    pub fn layout_plain(
        &mut self,
        segments: &Vec<Segment>,
        layout_style: &LayoutStyle,
        initial_text_style: &TextStyle,
        color_space: ColorSpace,
    ) -> Result<(Vec<GlyphVertices>, Vec<SegmentGlyphSpan>, u32, u32), String> {
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
    ) -> (Vec<GlyphVertices>, Vec<SegmentGlyphSpan>, u32, u32) {
        let initial_text_style = text_spans
            .as_ref()
            .first()
            .and_then(|span| span.runs.first())
            .map(|run| run.style.clone())
            .unwrap_or_default();
        let input =
            HuoziTiqianInputAdapter::adapt(text_spans.as_ref(), layout_style, &initial_text_style);
        let result = self.layout_engine.layout(input.layout_input);
        HuoziTiqianOutputAdapter::adapt(self, &result, &input.source_map, &color_space)
    }
}
