use proc_macro2::{Ident, Span};
use quote::{format_ident, quote, ToTokens};
use syn::spanned::Spanned;
use syn::{ItemFn, LitStr, Type};

use xee_xpath_ast::ast::Signature;
use xot::xmlname::NameStrInfo;

use crate::convert::convert_sequence_type;
use crate::parse::XPathFnOptions;

pub(crate) fn xpath_fn_wrapper(
    ast: &ItemFn,
    options: &XPathFnOptions,
) -> syn::Result<proc_macro2::TokenStream> {
    let name = &ast.sig.ident;
    let wrapper_name = format_ident!("wrapper_{}", name);
    let wrapper = make_wrapper(name, &wrapper_name, ast, &options.signature)?;

    let vis = &ast.vis;
    let signature_string = LitStr::new(&options.signature_string, Span::call_site());
    let kind = if let Some(kind) = &options.kind {
        LitStr::new(kind, Span::call_site())
    } else {
        LitStr::new("", Span::call_site())
    };
    Ok(quote! {
        // create a module with the same name as the function - this way `use
        // <the function> will bring both the function and module into scope.
        // This module contains information about the wrapper function
        // we access with the wrap_xpath_fn! macro.
        #[doc(hidden)]
        #vis mod #name {
            pub(crate) struct MakeWrapper;
            pub(crate) const WRAPPER: crate::function::StaticFunctionType = MakeWrapper::WRAPPER;
            // We store the signature as a string; this means we need to
            // reparse it again later during registration, but it's a lot
            // easier than trying to serialize a data structure, so it will
            // do for now.
            pub(crate) const SIGNATURE: &str = #signature_string;
            pub(crate) const KIND: &str = #kind;
        }

        // Generate the function inside of the same scope at the original
        // function (but in an isolated block), so that it can easily call the
        // original function. Using `super` isn't useful for that, as the
        // original function may be inside of a function body.
        const _: () = {
            // This is a trick to ensure we can get it into the module defined
            // above
            impl #name::MakeWrapper {
                const WRAPPER: crate::function::StaticFunctionType = #wrapper_name;
            }
            #vis #wrapper
        };
    })
}

fn make_wrapper(
    name: &Ident,
    wrapper_name: &Ident,
    ast: &ItemFn,
    signature: &Signature,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut conversions = Vec::new();
    let mut conversion_names = Vec::new();
    let mut adjust = 0;
    let context_ident = get_argument_ident(ast, adjust, "context")?;
    if let Some(context_ident) = context_ident {
        conversion_names.push(context_ident);
        adjust += 1
    }
    let interpreter_ident = get_argument_ident(ast, adjust, "interpreter")?;
    if let Some(interpreter_ident) = interpreter_ident {
        conversion_names.push(interpreter_ident);
        adjust += 1;
    }

    let expected_inputs = signature.params.len() + adjust;
    if ast.sig.inputs.len() != expected_inputs {
        let sig_count = signature.params.len();
        let rust_user_count = ast.sig.inputs.len().saturating_sub(adjust);
        let injected_note = match adjust {
            0 => String::new(),
            n => format!(" (after subtracting {n} injected `context`/`interpreter` argument(s))"),
        };
        bail_spanned!(
            ast.sig.ident.span() =>
            format!(
                "#[xpath_fn] arity mismatch: signature declares {sig_count} XPath parameter(s), \
                 but the Rust function takes {rust_user_count}{injected_note}"
            )
        );
    }

    for (i, param) in signature.params.iter().enumerate() {
        let name = Ident::new(param.name.local_name(), Span::call_site());
        conversion_names.push(name.clone());
        let arg = quote!(arguments[#i]);
        let fn_arg = &ast.sig.inputs[i + adjust];
        conversions.push(convert_sequence_type(
            &param.type_,
            fn_arg,
            name.to_token_stream(),
            arg,
        )?);
    }

    let body = if is_result(ast) {
        quote!(#(#conversions)*;
        let value = #name(#(#conversion_names),*);
        value.map(|v| v.into()))
    } else {
        quote!(#(#conversions)*;
        let value = #name(#(#conversion_names),*);
        Ok(value.into()))
    };

    Ok(quote!(
        fn #wrapper_name(context: &crate::context::DynamicContext, interpreter: &mut crate::interpreter::Interpreter, arguments: &[crate::sequence::Sequence]) -> Result<crate::sequence::Sequence, crate::error::Error> {
        #body
    }))
}

fn get_argument_ident(ast: &ItemFn, index: usize, name: &str) -> syn::Result<Option<Ident>> {
    if index >= ast.sig.inputs.len() {
        return Ok(None);
    }

    if !ast.sig.inputs.is_empty() {
        let maybe_context_arg = &ast.sig.inputs[index];
        match &maybe_context_arg {
            syn::FnArg::Typed(pat_type) => match &*pat_type.pat {
                syn::Pat::Ident(ident) => Ok(if ident.ident == name {
                    Some(ident.ident.clone())
                } else {
                    None
                }),
                _ => {
                    bail_spanned!(pat_type.span() => "XPath functions can only take identifiers as arguments");
                }
            },
            syn::FnArg::Receiver(r) => {
                bail_spanned!(r.span() => "XPath functions cannot take `self` as an argument");
            }
        }
    } else {
        Ok(None)
    }
}

fn is_result(ast: &ItemFn) -> bool {
    let return_type = &ast.sig.output;
    match return_type {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, type_) => match type_.as_ref() {
            Type::Path(type_path) => {
                matches!(
                    type_path
                        .path
                        .segments
                        .last()
                        .unwrap()
                        .ident
                        .to_string()
                        .as_str(),
                    "Result"
                )
            }
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use syn::parse_str;

    #[test]
    fn test_wrapper() {
        let options =
            parse_str::<XPathFnOptions>(r#""fn:foo($x as xs:int) as xs:string""#).unwrap();
        let ast = parse_str::<ItemFn>(
            r#"
            fn foo(x: &i64) -> String {
                format!("{}", x)
            }"#,
        )
        .unwrap();
        assert_debug_snapshot!(xpath_fn_wrapper(&ast, &options).unwrap().to_string());
    }

    #[test]
    fn test_wrapper_items_sequence_arg() {
        let options =
            parse_str::<XPathFnOptions>(r#""fn:foo($x as item()*) as xs:string""#).unwrap();
        let ast = parse_str::<ItemFn>(
            r#"
            fn foo(x: &crate::Sequence) -> String {
                "foo".to_string()
            }"#,
        )
        .unwrap();
        assert_debug_snapshot!(xpath_fn_wrapper(&ast, &options).unwrap().to_string());
    }

    #[test]
    fn test_wrapper_too_few_rust_args_errors() {
        // signature declares two XPath parameters but the Rust fn only
        // takes one — used to panic with index-out-of-bounds; now should
        // surface as a clean syn::Error pointing at the fn name.
        let options =
            parse_str::<XPathFnOptions>(r#""fn:foo($x as xs:int, $y as xs:int) as xs:string""#)
                .unwrap();
        let ast = parse_str::<ItemFn>(r#"fn foo(x: &i64) -> String { format!("{}", x) }"#).unwrap();
        assert_debug_snapshot!(xpath_fn_wrapper(&ast, &options).unwrap_err().to_string());
    }

    #[test]
    fn test_wrapper_too_many_rust_args_errors() {
        // Rust fn has one more arg than the signature declares.
        // The old code silently dropped the extra arg; we now reject.
        let options =
            parse_str::<XPathFnOptions>(r#""fn:foo($x as xs:int) as xs:string""#).unwrap();
        let ast = parse_str::<ItemFn>(r#"fn foo(x: &i64, y: &i64) -> String { format!("{}", x) }"#)
            .unwrap();
        assert_debug_snapshot!(xpath_fn_wrapper(&ast, &options).unwrap_err().to_string());
    }

    #[test]
    fn test_wrapper_arity_mismatch_with_injected_context() {
        // Even with context injection, arity is checked correctly:
        // signature 0 params + context injected = 1 Rust arg expected,
        // but Rust fn has 2.
        let options = parse_str::<XPathFnOptions>(r#""fn:foo() as xs:string""#).unwrap();
        let ast = parse_str::<ItemFn>(
            r#"fn foo(context: &DynamicContext, extra: &i64) -> String { String::new() }"#,
        )
        .unwrap();
        assert_debug_snapshot!(xpath_fn_wrapper(&ast, &options).unwrap_err().to_string());
    }
}
