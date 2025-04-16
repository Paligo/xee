use xee_xpath::error::Error;

pub(crate) fn render_error(src: &str, e: Error) {
    let red = ariadne::Color::Red;

    let mut report = ariadne::Report::build(ariadne::ReportKind::Error, ("source", (0..0)))
        .with_code(e.error.code());

    if let Some(span) = e.span {
        report = report.with_label(
            ariadne::Label::new(("source", span.range()))
                .with_message(e.error.message())
                .with_color(red),
        )
    }
    report
        .finish()
        .eprint(("source", ariadne::Source::from(src)))
        .unwrap();
    println!("{}", e.error.note());
}

pub(crate) fn render_parse_error(src: &str, e: xot::ParseError) {
    let red = ariadne::Color::Red;
    let mut report = ariadne::Report::build(ariadne::ReportKind::Error, ("source", (0..0)));

    report = report.with_label(
        ariadne::Label::new(("source", e.span().range()))
            .with_message(e)
            .with_color(red),
    );

    report
        .finish()
        .eprint(("source", ariadne::Source::from(src)))
        .unwrap();
}
