#![doc(hidden)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use venial::{Item, parse_item};

enum Error {
    Venial(venial::Error),
    Custom(String),
}

impl Error {
    fn to_compile_error(&self) -> TokenStream2 {
        match self {
            Self::Venial(error) => error.to_compile_error(),
            Self::Custom(error) => quote! {
                ::core::compile_error!(#error);
            },
        }
    }
}

impl From<venial::Error> for Error {
    fn from(value: venial::Error) -> Self {
        Self::Venial(value)
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self::Custom(value.into())
    }
}

fn derive_span_impl(item: TokenStream2) -> Result<TokenStream2, Error> {
    match parse_item(item)? {
        Item::Enum(e) => {
            let mut range_ts = TokenStream2::new();
            let mut offset_ts = TokenStream2::new();
            for var in e.variants.items().map(|var| &var.name) {
                range_ts.extend(quote! {
                    Self:: #var (v) => v.range(),
                });
                offset_ts.extend(quote! {
                    Self:: #var (v) => v.offset(offset),
                });
            }

            let g = e.generic_params;
            let name = e.name;
            Ok(quote! {
                impl #g crate::lex::Span for #name #g {
                    fn range(&self) -> ::core::ops::Range<usize> {
                        match self {
                            #range_ts
                        }
                    }

                    fn offset(&mut self, offset: usize) {
                        match self {
                            #offset_ts
                        }
                    }
                }
            })
        }
        Item::Struct(s) => {
            let g = s.generic_params;
            let name = s.name;
            Ok(quote! {
                impl #g crate::lex::Span for #name #g {
                    fn range(&self) -> ::core::ops::Range<usize> {
                        self.range.clone()
                    }

                    fn offset(&mut self, offset: usize) {
                        self.range.start += offset;
                        self.range.end += offset;
                    }
                }
            })
        }
        _ => Err("is not enum nor struct".into()),
    }
}

#[proc_macro_derive(Span)]
pub fn derive_span(item: TokenStream) -> TokenStream {
    match derive_span_impl(item.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
