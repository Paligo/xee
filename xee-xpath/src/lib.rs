#![warn(missing_docs)]

//! This module provides a high level API to use XPath from Rust.
//!
//! You can compile XPath expressions into queries using the [`Queries`] store.
//! For each query you supply a conversion function that turns the result XPath
//! sequence into a Rust value.
//!
//! You can create a [`Documents`] store and load XML documents into it.
//!
//! You can also add queries to a [`Queries`] object. You can
//! then execute queries against the documents store.
//!
//! ```rust
//! use xee_xpath::{Documents, Queries, Query};
//!
//! // create a new documents object
//! let mut documents = Documents::new();
//! // load a document from a string
//! let doc = documents.add_string("http://example.com".try_into().unwrap(), "<root>foo</root>").unwrap();
//!
//! // create a new queries object
//! let queries = Queries::default();
//!
//! // create a query expecting a single value in the result sequence
//! // try to convert this value into a Rust `String`
//! let q = queries.one("/root/string()", |_, item| {
//!   Ok(item.try_into_value::<String>()?)
//! })?;
//!
//! // when we execute the query, we need to pass a mutable reference to the documents,
//! // and the item against which we want to query. We can also pass in a document handle,
//! // as we do here
//! let r = q.execute(&mut documents, doc)?;
//! assert_eq!(r, "foo");
//!
//! # Ok::<(), xee_xpath::error::Error>(())
//! ```
//!
//! Note that to represent URLs, we use the
//! [`iri-string`](https://docs.rs/iri-string/latest/iri_string/) crate.
//! To make an `IriString` from a string, you can use the `try_into` method:
//!
//! ```rust
//! use iri_string::types::IriString;
//!
//! let uri: IriString = "http://example.com".try_into().unwrap();
//! # Ok::<(), xee_xpath::error::Error>(())
//! ```
//!
//! To make an `IriStr` reference, just use `&` on an `IriString`; this is
//! like the relationship between `String` and `&str`. You can also use the
//! `try_into` method on a `&str` directly:
//!
//! ```rust
//! use iri_string::types::IriStr;
//!
//! let uri: &IriStr = "http://example.com".try_into().unwrap();
//! # Ok::<(), xee_xpath::error::Error>(())
//! ```

pub mod atomic;
pub mod context;
mod documents;
pub mod error;
pub mod function;
mod itemable;
pub mod iter;
mod queries;
pub mod query;

pub use documents::Documents;
pub use itemable::Itemable;
pub use queries::Queries;
pub use query::{Query, Recurse};
pub use xee_interpreter::atomic::Atomic;
pub use xee_interpreter::sequence::{Item, Sequence};
pub use xee_interpreter::xml::DocumentHandle;
