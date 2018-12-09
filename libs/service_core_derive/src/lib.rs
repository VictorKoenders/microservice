// This is always necessary to get the `TokenStream` typedef.
extern crate proc_macro;
#[macro_use]
extern crate serde_derive;
extern crate serde;

use proc_macro::{Ident, Literal, TokenStream, TokenTree};
use quote::quote;
use reqwest;
use semver::Version;
use std::net::SocketAddr;

#[derive(Default)]
struct ServiceDefinitionBuilder {
    pub name: Option<String>,
    pub version: Option<Version>,
}

#[derive(Debug)]
struct ServiceDefinition {
    pub name: String,
    pub version: Version,
}

#[derive(Debug, Serialize, Deserialize, Eq, Hash, PartialEq, Clone)]
struct ServiceName {
    pub name: String,
    pub version: Version,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Service {
    pub name: ServiceName,
    pub address: SocketAddr,
    pub methods: Vec<ServiceMethod>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ServiceMethod {
    pub name: String,
    pub args: Vec<ServiceMethodArgument>,
    pub returning: Type,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ServiceMethodArgument {
    pub name: String,
    pub r#type: Type,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Type(pub String);

impl From<ServiceDefinitionBuilder> for ServiceDefinition {
    fn from(d: ServiceDefinitionBuilder) -> ServiceDefinition {
        ServiceDefinition {
            name: d.name.expect("Missing field: \"name\""),
            version: d.version.expect("Missing field: \"version\""),
        }
    }
}

#[proc_macro_attribute]
pub fn mock_service(args: TokenStream, _input: TokenStream) -> TokenStream {
    let mut iter = args.into_iter().peekable();
    let mut definition = ServiceDefinitionBuilder::default();
    while iter.peek().is_some() {
        let name = get_ident(&mut iter).to_string();
        consume_punct(&mut iter, '=');
        match name.as_str() {
            "name" => {
                let lit = get_lit(&mut iter).to_string();
                let name = lit.trim_matches('"');
                definition.name = Some(name.to_string());
            }
            "version" => {
                let lit = get_lit(&mut iter).to_string();
                let version = lit.trim_matches('"');
                definition.version = Some(version.parse().expect("Could not parse version string"));
            }
            x => panic!("Unknown tag: {:?}", x),
        }

        if let Some(next) = iter.peek() {
            if let TokenTree::Punct(p) = next {
                if p.as_char() == ',' {
                    iter.next();
                }
            }
        }
    }

    let definition: ServiceDefinition = definition.into();

    let data: Service = reqwest::get(&format!(
        "http://localhost:8000/api/service/{0}/{1}",
        definition.name, definition.version
    ))
    .expect("Could not query index service")
    .json()
    .expect("Could not deserialize service definition");

    let methods = data.methods.iter().map(|m| {
        let name = syn::Ident::new(m.name.as_str(), proc_macro2::Span::call_site());
        let returning = syn::Ident::new(m.returning.0.as_str(), proc_macro2::Span::call_site());
        quote! {
            pub fn #name() -> #returning {
                unimplemented!()
            }
        }
    });
    let name = syn::Ident::new(data.name.name.as_str(), proc_macro2::Span::call_site());
    let result = quote! {
        pub mod #name {
            #(#methods)*
        }
    };
    println!("{}", result);
    result.into()
}

fn consume_punct(iter: &mut Iterator<Item = TokenTree>, c: char) {
    match iter.next() {
        Some(TokenTree::Punct(p)) => {
            if p.as_char() != c {
                panic!("Expected token '{}', got '{}'", c, p.as_char());
            }
        }
        x => {
            panic!("Expected token '{}', got {:?}", c, x);
        }
    }
}

fn get_lit(iter: &mut Iterator<Item = TokenTree>) -> Literal {
    match iter.next() {
        Some(TokenTree::Literal(literal)) => literal,
        x => panic!("Expected literal, got {:?}", x),
    }
}

fn get_ident(iter: &mut Iterator<Item = TokenTree>) -> Ident {
    match iter.next() {
        Some(TokenTree::Ident(ident)) => ident,
        x => panic!("Expected ident, got {:?}", x),
    }
}
