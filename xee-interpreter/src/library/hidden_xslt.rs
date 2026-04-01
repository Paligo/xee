// functions used to implement the XSLT that aren't supposed to be
// exposed to XPath
use std::collections::HashSet;

use xee_xpath_macros::xpath_fn;
use xot::Xot;

use crate::atomic;
use crate::error;
use crate::function::StaticFunctionDescription;
use crate::interpreter::Interpreter;
use crate::sequence;
use crate::wrap_xpath_fn;

// TODO: Things should really be hidden from XPath, and not be in the fn prefix

// https://www.w3.org/TR/xslt-30/#constructing-simple-content

#[xpath_fn("fn:simple-content($arg as item()*, $separator as xs:string) as xs:string?")]
fn simple_content(
    interpreter: &Interpreter,
    arg: &sequence::Sequence,
    separator: &str,
) -> error::Result<String> {
    let arg = simple_content_text_nodes(arg, interpreter.xot())?;
    // now atomize the sequence, putting in separators, except at the end
    let mut s = String::new();
    let mut first = true;
    for atom in arg.atomized(interpreter.xot()) {
        let atom = atom?;
        if !first {
            s.push_str(separator);
        }
        s.push_str(&atom.into_canonical());
        first = false;
    }
    Ok(s)
}

#[xpath_fn("fn:group-by-first($seq as item()*, $key as function(item()) as xs:anyAtomicType*) as item()*")]
fn group_by_first(
    interpreter: &mut Interpreter,
    seq: &sequence::Sequence,
    key: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let function = key.to_function()?;
    let mut seen: HashSet<atomic::Atomic> = HashSet::new();
    let mut result = Vec::new();

    for item in seq.iter() {
        let value = interpreter.call_function_with_arguments(&function, &[item.clone().into()])?;
        let keys = value
            .atomized(interpreter.xot())
            .collect::<error::Result<Vec<_>>>()?;
        if keys.into_iter().any(|atomic| seen.insert(atomic)) {
            result.push(item.clone());
        }
    }

    Ok(result.into())
}

fn simple_content_text_nodes(
    arg: &sequence::Sequence,
    xot: &Xot,
) -> error::Result<sequence::Sequence> {
    // 1. zero-length text nodes in the sequence are discarded
    // 2. adjacent text nodes are merged into a single text node

    // Note: to avoid having to create xot nodes on the fly, we actually
    // turn adjecent text nodes into atomic string nodes, which should be
    // fine.
    let mut r: Vec<sequence::Item> = Vec::new();
    let mut last_text: Option<String> = None;
    for item in arg.iter() {
        if let sequence::Item::Node(node) = item {
            if let xot::Value::Text(text) = xot.value(node) {
                let text = text.get();
                if text.is_empty() {
                    continue;
                }
                if let Some(mut s) = last_text.take() {
                    // add the text to the last text node
                    s.push_str(text);
                    last_text = Some(s);
                } else {
                    // set the last text node instead
                    last_text = Some(text.to_string());
                }
                continue;
            }
        }
        if let Some(s) = last_text.take() {
            r.push(sequence::Item::Atomic(s.into()));
        }
        r.push(item.clone());
    }
    // set the last text node
    if let Some(s) = last_text.take() {
        r.push(sequence::Item::Atomic(s.into()));
    }
    Ok(r.into())
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![wrap_xpath_fn!(simple_content), wrap_xpath_fn!(group_by_first)]
}

#[cfg(test)]
mod tests {
    use super::*;

    use sequence::{Item, Sequence};

    #[test]
    fn test_filter_empty_text_nodes() {
        let mut xot = Xot::new();
        let empty = xot.new_text("");

        let sequence = Sequence::from(vec![
            Item::Node(empty),
            Item::Atomic(1.into()),
            Item::Node(empty),
            Item::Atomic(2.into()),
            Item::Node(empty),
        ]);
        let result = simple_content_text_nodes(&sequence, &xot).unwrap();
        assert_eq!(result.len(), 2);
        let items = result.iter().collect::<Vec<_>>();
        assert_eq!(items, vec![Item::Atomic(1.into()), Item::Atomic(2.into())]);
    }

    #[test]
    fn test_concatenate_adjacent_text_nodes() {
        let mut xot = Xot::new();
        let a = xot.new_text("a");
        let b = xot.new_text("b");
        let sequence = Sequence::from(vec![Item::Node(a), Item::Node(b)]);
        let result = simple_content_text_nodes(&sequence, &xot).unwrap();
        let items = result.iter().collect::<Vec<_>>();
        assert_eq!(items, vec![Item::Atomic("ab".into())]);
    }

    #[test]
    fn test_concatenate_adjacent_text_nodes_three() {
        let mut xot = Xot::new();
        let a = xot.new_text("a");
        let b = xot.new_text("b");
        let c = xot.new_text("c");

        let sequence = Sequence::from(vec![Item::Node(a), Item::Node(b), Item::Node(c)]);
        let result = simple_content_text_nodes(&sequence, &xot).unwrap();
        let items = result.iter().collect::<Vec<_>>();
        assert_eq!(items, vec![Item::Atomic("abc".into())]);
    }

    #[test]
    fn test_concatenate_adjacent_text_nodes_not_ending() {
        let mut xot = Xot::new();
        let a = xot.new_text("a");
        let b = xot.new_text("b");
        let c = xot.new_text("c");

        let sequence = Sequence::from(vec![
            Item::Node(a),
            Item::Node(b),
            Item::Node(c),
            Item::Atomic(1.into()),
        ]);
        let result = simple_content_text_nodes(&sequence, &xot).unwrap();
        let items = result.iter().collect::<Vec<_>>();
        assert_eq!(
            items,
            vec![Item::Atomic("abc".into()), Item::Atomic(1.into())]
        );
    }
}
