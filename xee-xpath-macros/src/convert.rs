// In this module, we generate quoted code that converts an incoming
// Sequence into the required Rust type, using the SequenceType as a guide.

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

use xee_schema_type::Xs;
use xee_xpath_ast::ast;

pub(crate) fn convert_sequence_type(
    sequence_type: &ast::SequenceType,
    fn_arg: &syn::FnArg,
    name: TokenStream,
    arg: TokenStream,
) -> syn::Result<TokenStream> {
    match sequence_type {
        ast::SequenceType::Empty => Ok(quote!(
            #[allow(non_snake_case)]
            let #name = #arg.ensure_empty()?;
        )),
        ast::SequenceType::Item(item) => convert_item(item, fn_arg, name, arg),
    }
}

fn convert_item(
    item: &ast::Item,
    fn_arg: &syn::FnArg,
    name: TokenStream,
    arg: TokenStream,
) -> syn::Result<TokenStream> {
    let (iterator, borrow) = convert_item_type(&item.item_type, fn_arg, arg.clone())?;

    Ok(match &item.occurrence {
        ast::Occurrence::One => {
            let as_ref = if borrow {
                quote!(
                    #[allow(non_snake_case)]
                    let #name = #name.as_ref();
                )
            } else {
                quote!()
            };
            quote!(
                #[allow(non_snake_case)]
                let #name = crate::occurrence::one(&mut #iterator)?;
                #as_ref
            )
        }
        ast::Occurrence::Option => {
            let as_ref = if borrow {
                quote!(
                    #[allow(non_snake_case)]
                    let #name = #name.as_deref();
                )
            } else {
                quote!()
            };
            quote!(
                #[allow(non_snake_case)]
                let #name = crate::occurrence::option(&mut #iterator)?;
                #as_ref
            )
        }
        ast::Occurrence::Many => {
            if is_sequence_arg(fn_arg) {
                // we already have a reference argument, so
                // we don't need to do anything to it
                return Ok(quote!(
                    #[allow(non_snake_case)]
                    let #name = &(#arg);
                ));
            }
            // let name_temp =
            //     syn::Ident::new(&format!("tmp_{}", name), proc_macro2::Span::call_site());
            // let as_ref = if borrow {
            //     quote!(
            //         #[allow(non_snake_case)]
            //         let #name_temp = #name_temp.iter().map(|s| s.as_ref()).collect::<Vec<_>>();
            //     )
            // } else {
            //     quote!()
            // };
            quote!(
                #[allow(non_snake_case)]
                let mut #name = #iterator;
            )
            // let many = quote!(#occurrence::many(&mut #iterator)?);
            // quote!(
            //     #[allow(non_snake_case)]
            //     let #name_temp = #many;
            //     #as_ref
            //     #[allow(non_snake_case)]
            //     let #name = #name_temp.as_slice();
            // )
        }
        ast::Occurrence::NonEmpty => bail_spanned!(
            fn_arg.span() =>
            "one-or-more occurrence (`+`) is not yet supported in #[xpath_fn] signatures"
        ),
    })
}

fn convert_item_type(
    item: &ast::ItemType,
    fn_arg: &syn::FnArg,
    arg: TokenStream,
) -> syn::Result<(TokenStream, bool)> {
    match item {
        ast::ItemType::Item => Ok((quote!(#arg.iter().map(Ok)), false)),
        ast::ItemType::AtomicOrUnionType(xs) => {
            let (token_stream, borrow) = convert_atomic_or_union_type(*xs, fn_arg, arg)?;
            Ok((token_stream, borrow))
        }
        ast::ItemType::KindTest(kind_test) => {
            Ok((convert_kind_test(kind_test, fn_arg, arg)?, false))
        }
        // we don't do anything special for higher order functions at this point;
        // the implementation is supposed to manually unpack the items
        ast::ItemType::FunctionTest(_) => Ok((quote!(#arg.iter().map(Ok)), false)),
        ast::ItemType::ArrayTest(array_test) => match array_test {
            ast::ArrayTest::AnyArrayTest => Ok((quote!(#arg.array_iter()), false)),
            _ => bail_spanned!(
                fn_arg.span() =>
                "typed array tests (e.g. `array(xs:integer)`) are not yet supported in #[xpath_fn] signatures — use `array(*)`"
            ),
        },
        ast::ItemType::MapTest(map_test) => match map_test {
            ast::MapTest::AnyMapTest => Ok((quote!(#arg.map_iter()), false)),
            _ => bail_spanned!(
                fn_arg.span() =>
                "typed map tests (e.g. `map(xs:string, xs:integer)`) are not yet supported in #[xpath_fn] signatures — use `map(*)`"
            ),
        },
    }
}

fn convert_atomic_or_union_type(
    xs: Xs,
    fn_arg: &syn::FnArg,
    arg: TokenStream,
) -> syn::Result<(TokenStream, bool)> {
    if xs == Xs::AnyAtomicType || xs == Xs::Numeric {
        return Ok((quote!(#arg.atomized(interpreter.xot())), false));
    }

    let Some(rust_info) = xs.rust_info() else {
        bail_spanned!(
            fn_arg.span() =>
            format!(
                "XPath atomic type `{xs:?}` has no Rust wrapper — cannot bridge it through #[xpath_fn]. \
                 If this type should be supported, register it in xee_schema_type::Xs::rust_info"
            )
        );
    };
    let type_name = rust_info.rust_name();
    let type_name = syn::parse_str::<syn::Type>(type_name)?;
    let convert = quote!(std::convert::TryInto::<#type_name>::try_into(atomic));

    let borrow = rust_info.is_reference();
    Ok((
        quote!(#arg.unboxed_atomized(interpreter.xot(), |atomic| #convert)),
        borrow,
    ))
}

fn convert_kind_test(
    kind_test: &ast::KindTest,
    fn_arg: &syn::FnArg,
    arg: TokenStream,
) -> syn::Result<TokenStream> {
    match kind_test {
        ast::KindTest::Any => Ok(quote!(#arg.nodes())),
        ast::KindTest::Element(element_test) => {
            if element_test.is_some() {
                bail_spanned!(
                    fn_arg.span() =>
                    "named element tests (e.g. `element(foo)`) are not yet supported in #[xpath_fn] signatures — use `element()`"
                );
            }
            Ok(quote!(#arg.elements(interpreter.xot())?))
        }
        _ => bail_spanned!(
            fn_arg.span() =>
            "this kind test is not yet supported in #[xpath_fn] signatures — only `node()`, `element()`, and `item()` work today"
        ),
    }
}

fn is_sequence_arg(fn_arg: &syn::FnArg) -> bool {
    match fn_arg {
        syn::FnArg::Receiver(_) => false,
        syn::FnArg::Typed(type_) => match type_.ty.as_ref() {
            syn::Type::Reference(type_) => match type_.elem.as_ref() {
                syn::Type::Path(type_) => {
                    let segment = type_.path.segments.iter().last();
                    match segment {
                        Some(syn::PathSegment {
                            ident,
                            arguments: _arguments,
                        }) => ident == "Sequence",
                        _ => false,
                    }
                }
                _ => false,
            },
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use xee_xpath_ast::parse_sequence_type;
    use xee_xpath_ast::Namespaces;

    fn convert(s: &str) -> String {
        // dummy fixed fn arg here
        convert_fn_arg(s, &syn::parse_str("a: &str").unwrap())
    }

    fn convert_fn_arg(s: &str, fn_arg: &syn::FnArg) -> String {
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type(s, &namespaces).unwrap();
        let name = quote!(a);
        let arg = quote!(arguments[0]);

        convert_sequence_type(&sequence_type, fn_arg, name, arg)
            .unwrap()
            .to_string()
    }

    #[test]
    fn test_convert() {
        assert_debug_snapshot!(convert("xs:integer"));
    }

    #[test]
    fn test_convert_option() {
        assert_debug_snapshot!(convert("xs:integer?"));
    }

    #[test]
    fn test_convert_many() {
        assert_debug_snapshot!(convert("xs:integer*"));
    }

    #[test]
    fn test_convert_empty_sequence() {
        assert_debug_snapshot!(convert("empty-sequence()"));
    }

    #[test]
    fn test_convert_item() {
        assert_debug_snapshot!(convert("item()"));
    }

    #[test]
    fn test_convert_any_atomic_type() {
        assert_debug_snapshot!(convert("xs:anyAtomicType"));
    }

    #[test]
    fn test_convert_node() {
        assert_debug_snapshot!(convert("node()"));
    }

    #[test]
    fn test_convert_string() {
        assert_debug_snapshot!(convert("xs:string"));
    }

    #[test]
    fn test_convert_string_option() {
        assert_debug_snapshot!(convert("xs:string?"));
    }

    #[test]
    fn test_convert_string_many() {
        assert_debug_snapshot!(convert("xs:string*"));
    }

    #[test]
    fn test_convert_sequence_arg() {
        assert_debug_snapshot!(convert_fn_arg(
            "item()*",
            &syn::parse_str("a: &crate::Sequence").unwrap()
        ));
    }

    #[test]
    fn test_convert_array() {
        assert_debug_snapshot!(convert("array(*)"));
    }

    fn convert_err(s: &str) -> String {
        let fn_arg: syn::FnArg = syn::parse_str("a: &str").unwrap();
        let namespaces = Namespaces::default();
        let sequence_type = parse_sequence_type(s, &namespaces).unwrap();
        let name = quote!(a);
        let arg = quote!(arguments[0]);

        convert_sequence_type(&sequence_type, &fn_arg, name, arg)
            .expect_err("expected a syn::Error from convert_sequence_type")
            .to_string()
    }

    #[test]
    fn test_convert_nonempty_occurrence_errors() {
        assert_debug_snapshot!(convert_err("xs:integer+"));
    }

    #[test]
    fn test_convert_named_element_test_errors() {
        // element(foo) — named element tests aren't supported yet
        assert_debug_snapshot!(convert_err("element(foo)"));
    }

    #[test]
    fn test_convert_attribute_kind_test_errors() {
        assert_debug_snapshot!(convert_err("attribute()"));
    }

    #[test]
    fn test_convert_text_kind_test_errors() {
        assert_debug_snapshot!(convert_err("text()"));
    }

    #[test]
    fn test_convert_typed_array_errors() {
        assert_debug_snapshot!(convert_err("array(xs:integer)"));
    }

    #[test]
    fn test_convert_typed_map_errors() {
        assert_debug_snapshot!(convert_err("map(xs:string, xs:integer)"));
    }
}
