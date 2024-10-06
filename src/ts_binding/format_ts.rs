use biome_formatter::{IndentStyle, IndentWidth, LineWidth, QuoteStyle};
use biome_js_formatter::{
    context::{JsFormatOptions, Semicolons},
    format_node,
};
use biome_js_parser::{parse, JsParserOptions};
use biome_js_syntax::JsFileSource;

pub fn format_ts_string(text: &str) -> Result<String, anyhow::Error> {
    let source_type = JsFileSource::ts();
    let tree = parse(text, source_type, JsParserOptions::default());

    let format_options = JsFormatOptions::new(source_type)
        .with_indent_style(IndentStyle::Space)
        .with_line_width(LineWidth::try_from(80).unwrap())
        .with_semicolons(Semicolons::Always)
        .with_quote_style(QuoteStyle::Double)
        .with_indent_width(IndentWidth::from(4));

    let doc = format_node(format_options, &tree.syntax())?;
    let result = doc.print()?.as_code().to_string();
    Ok(result)
}
