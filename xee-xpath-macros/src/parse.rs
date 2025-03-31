use syn::{
    parse::{Parse, ParseStream},
    token::Comma,
    LitStr, Result,
};

use xee_xpath_ast::ast::Signature;
use xee_xpath_ast::Namespaces;

#[derive(Debug)]
pub(crate) struct XPathFnOptions {
    pub(crate) signature: Signature,
    pub(crate) kind: Option<String>,
    pub(crate) signature_string: String,
}

mod kw {
    syn::custom_keyword!(context_first);
    syn::custom_keyword!(context_last);
    syn::custom_keyword!(context_last_optional);
    syn::custom_keyword!(position);
    syn::custom_keyword!(size);
    syn::custom_keyword!(collation);
    syn::custom_keyword!(anonymous_closure);
}

impl Parse for XPathFnOptions {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut options = Vec::new();
        while !input.is_empty() {
            options.push(input.parse()?);
            if !input.is_empty() {
                let _: Comma = input.parse()?;
            }
        }

        let mut signature: Option<String> = None;
        let mut kind: Option<String> = None;
        for option in options {
            match option {
                XPathFnOption::Signature(signature_option) => {
                    signature = Some(signature_option);
                }
                XPathFnOption::Kind(kind_option) => {
                    kind = Some(kind_option);
                }
            }
        }
        let signature_string = signature.expect("Signature not found");
        let namespaces = Namespaces::default();
        let signature = Signature::parse(&signature_string, &namespaces)
            .map_err(|e| input.error(format!("{:?}", e)))?;
        Ok(Self {
            signature,
            kind,
            signature_string,
        })
    }
}

enum XPathFnOption {
    Signature(String),
    Kind(String),
}

impl Parse for XPathFnOption {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        Ok(if lookahead.peek(kw::context_first) {
            let _eat: kw::context_first = input.parse()?;
            XPathFnOption::Kind("context_first".to_string())
        } else if lookahead.peek(kw::context_last) {
            let _eat: kw::context_last = input.parse()?;
            XPathFnOption::Kind("context_last".to_string())
        } else if lookahead.peek(kw::context_last_optional) {
            let _eat: kw::context_last_optional = input.parse()?;
            XPathFnOption::Kind("context_last_optional".to_string())
        } else if lookahead.peek(kw::position) {
            let _eat: kw::position = input.parse()?;
            XPathFnOption::Kind("position".to_string())
        } else if lookahead.peek(kw::size) {
            let _eat: kw::size = input.parse()?;
            XPathFnOption::Kind("size".to_string())
        } else if lookahead.peek(kw::collation) {
            let _eat: kw::collation = input.parse()?;
            XPathFnOption::Kind("collation".to_string())
        } else if lookahead.peek(kw::anonymous_closure) {
            let _eat: kw::anonymous_closure = input.parse()?;
            XPathFnOption::Kind("anonymous_closure".to_string())
        } else if lookahead.peek(LitStr) {
            let string_literal: LitStr = input.parse()?;
            let signature = string_literal.value();
            XPathFnOption::Signature(signature)
        } else {
            bail_spanned!(
                input.span() => "Expected a string literal or a context keyword"
            );
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;

    #[test]
    fn test_parse_signature() {
        assert_debug_snapshot!(syn::parse_str::<XPathFnOptions>(
            r#""fn:foo() as xs:string""#
        ));
    }

    #[test]
    fn test_parse_signature_with_kind() {
        assert_debug_snapshot!(syn::parse_str::<XPathFnOptions>(
            r#""fn:foo() as xs:string", context_first"#
        ));
    }

    #[test]
    fn test_parse_signature_parse_error() {
        assert_debug_snapshot!(syn::parse_str::<XPathFnOptions>(r#""wrong wrong""#));
    }

    #[test]
    fn test_parse_not_a_signature_string() {
        assert_debug_snapshot!(syn::parse_str::<XPathFnOptions>(r#"wrong"#));
    }

    #[test]
    fn test_parse_signature_unknown_kind() {
        assert_debug_snapshot!(syn::parse_str::<XPathFnOptions>(
            r#""fn:foo() as xs:string",blah"#
        ));
    }
}
