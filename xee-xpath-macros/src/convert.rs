// In this module, we generate quoted code that converts an incoming
// Sequence into the required Rust type, using the SequenceType as a guide.

use proc_macro2::TokenStream;
use quote::quote;

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
    let (iterator, borrow) = convert_item_type(&item.item_type, arg.clone())?;
    let occurrence = quote!(crate::occurrence::Occurrence);

    Ok(match &item.occurrence {
        ast::Occurrence::One => {
            let as_ref = if borrow {
                quote!(let #name = #name.as_ref();)
            } else {
                quote!()
            };
            quote!(
                let #name = #occurrence::one(&mut #iterator)?;
                #as_ref
            )
        }
        ast::Occurrence::Option => {
            let as_ref = if borrow {
                quote!(let #name = #name.as_deref();)
            } else {
                quote!()
            };
            quote!(
                let #name = #occurrence::option(&mut #iterator)?;
                #as_ref
            )
        }
        ast::Occurrence::Many => {
            if is_sequence_arg(fn_arg) {
                // we already have a reference argument, so
                // we don't need to do anything to it
                return Ok(quote!(let #name = &(#arg);));
            }
            let name_temp =
                syn::Ident::new(&format!("tmp_{}", name), proc_macro2::Span::call_site());
            let as_ref = if borrow {
                quote!(let #name_temp = #name_temp.iter().map(|s| s.as_ref()).collect::<Vec<_>>();)
            } else {
                quote!()
            };
            let many = quote!(#occurrence::many(&mut #iterator)?);
            quote!(
                let #name_temp = #many;
                #as_ref
                let #name = #name_temp.as_slice();
            )
        }
        ast::Occurrence::NonEmpty => todo!("NonEmpty not yet supported"),
    })
}

fn convert_item_type(item: &ast::ItemType, arg: TokenStream) -> syn::Result<(TokenStream, bool)> {
    match item {
        ast::ItemType::Item => Ok((quote!(#arg.items()), false)),
        ast::ItemType::AtomicOrUnionType(xs) => {
            let (token_stream, borrow) = convert_atomic_or_union_type(*xs, arg)?;
            Ok((token_stream, borrow))
        }
        ast::ItemType::KindTest(kind_test) => Ok((convert_kind_test(kind_test, arg)?, false)),
        // we don't do anything special for higher order functions at this point;
        // the implementation is supposed to manually unpack the items
        ast::ItemType::FunctionTest(_) => Ok((quote!(#arg.items()), false)),
        ast::ItemType::ArrayTest(array_test) => match array_test {
            ast::ArrayTest::AnyArrayTest => Ok((quote!(#arg.array_iter()), false)),
            _ => todo!("Unsupported item type: typed array test"),
        },
        ast::ItemType::MapTest(map_test) => match map_test {
            ast::MapTest::AnyMapTest => Ok((quote!(#arg.map_iter()), false)),
            _ => todo!("Unsupported item type: typed map test"),
        },
    }
}

fn convert_atomic_or_union_type(xs: Xs, arg: TokenStream) -> syn::Result<(TokenStream, bool)> {
    if xs == Xs::AnyAtomicType || xs == Xs::Numeric {
        return Ok((quote!(#arg.atomized(context.xot())), false));
    }

    // TODO: another unwrap that should really be "we cannot create a rust wrapper
    // for this type" error
    let rust_info = xs
        .rust_info()
        .expect("Rust wrapper for this type not found");
    let type_name = rust_info.rust_name();
    let type_name = syn::parse_str::<syn::Type>(type_name)?;
    let convert = quote!(std::convert::TryInto::<#type_name>::try_into(atomic));

    let borrow = rust_info.is_reference();
    Ok((
        quote!(#arg.unboxed_atomized(context.xot(), |atomic| #convert)),
        borrow,
    ))
}

fn convert_kind_test(kind_test: &ast::KindTest, arg: TokenStream) -> syn::Result<TokenStream> {
    match kind_test {
        ast::KindTest::Any => Ok(quote!(#arg.nodes())),
        ast::KindTest::Element(element_test) => {
            if element_test.is_some() {
                unreachable!("Unsupported element test")
            }
            Ok(quote!(#arg.elements(context.xot())))
        }
        _ => {
            todo!("Unsupported kind test")
        }
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
}
