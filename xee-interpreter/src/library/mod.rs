/// XPath defines a standard function library, and this module implements
/// it.
mod accessor;
mod array;
mod boolean;
mod context;
mod datetime;
mod duration;
mod external;
mod fn_;
mod hidden_xslt;
mod hof;
mod id;
mod json;
mod map;
mod math;
mod node;
mod numeric;
mod parse;
mod qname;
mod sequence;
mod string;
mod uri;
mod xs;
mod rng;

use crate::function::StaticFunctionDescription;

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    let mut descriptions = Vec::new();
    descriptions.extend(fn_::static_function_descriptions());
    descriptions.extend(string::static_function_descriptions());
    descriptions.extend(xs::static_function_descriptions());
    descriptions.extend(numeric::static_function_descriptions());
    descriptions.extend(math::static_function_descriptions());
    descriptions.extend(boolean::static_function_descriptions());
    descriptions.extend(accessor::static_function_descriptions());
    descriptions.extend(duration::static_function_descriptions());
    descriptions.extend(datetime::static_function_descriptions());
    descriptions.extend(sequence::static_function_descriptions());
    descriptions.extend(node::static_function_descriptions());
    descriptions.extend(qname::static_function_descriptions());
    descriptions.extend(context::static_function_descriptions());
    descriptions.extend(hof::static_function_descriptions());
    descriptions.extend(array::static_function_descriptions());
    descriptions.extend(map::static_function_descriptions());
    descriptions.extend(hidden_xslt::static_function_descriptions());
    descriptions.extend(uri::static_function_descriptions());
    descriptions.extend(external::static_function_descriptions());
    descriptions.extend(parse::static_function_descriptions());
    descriptions.extend(json::static_function_descriptions());
    descriptions.extend(id::static_function_descriptions());
    descriptions.extend(rng::static_function_descriptions());
    descriptions
}
