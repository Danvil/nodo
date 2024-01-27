use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::Data;
use syn::DataStruct;
use syn::DeriveInput;
use syn::Fields;

/// Derive macro to implement the RxBundle trait for a custom struct with Rx fields
#[proc_macro_derive(RxBundleDerive)]
pub fn rx_bundle_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_rx_bundle_derive(&input)
}

fn impl_rx_bundle_derive(input: &syn::DeriveInput) -> TokenStream {
    let name = &input.ident;
    let name_str = name.to_string();

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let fields_count = fields.len();
    let field_index = (0..fields.len()).collect::<Vec<_>>();
    let field_name = fields.iter().map(|field| &field.ident).collect::<Vec<_>>();
    let field_name_str = fields
        .iter()
        .map(|f| f.ident.as_ref().unwrap().to_string())
        .collect::<Vec<_>>();

    let gen = quote! {
        impl nodo::channels::RxBundle for #name {
            fn name(&self, index: usize) -> String {
                match index {
                    #(#field_index => (#field_name_str).to_string(),)*
                    _ => panic!("invalid rx bundle index {index} for `{}`", #name_str),
                }
            }

            fn sync(&mut self) {
                use nodo::channels::Rx;

                #(self.#field_name.sync();)*
            }

            fn check_connection(&self) -> nodo::channels::ConnectionCheck {
                use nodo::channels::Rx;

                let mut cc = nodo::channels::ConnectionCheck::new(#fields_count);
                #(cc.mark(#field_index, self.#field_name.is_connected());)*
                cc
            }
        }
    };
    gen.into()
}

/// Derive macro to implement the TxBundle trait for a custom struct with Tx fields
#[proc_macro_derive(TxBundleDerive)]
pub fn tx_bundle_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_tx_bundle_derive(&input)
}

fn impl_tx_bundle_derive(input: &syn::DeriveInput) -> TokenStream {
    let name = &input.ident;
    let name_str = name.to_string();

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    let fields_count = fields.len();
    let field_index = (0..fields.len()).collect::<Vec<_>>();
    let field_name = fields.iter().map(|field| &field.ident).collect::<Vec<_>>();
    let field_name_str = fields
        .iter()
        .map(|f| f.ident.as_ref().unwrap().to_string())
        .collect::<Vec<_>>();

    let gen = quote! {
        impl nodo::channels::TxBundle for #name {
            fn name(&self, index: usize) -> String {
                match index {
                    #(#field_index => (#field_name_str).to_string(),)*
                    _ => panic!("invalid tx bundle index {index} for `{}`", #name_str),
                }
            }

            fn flush(&mut self) -> Result<(), nodo::channels::MultiFlushError> {
                use nodo::channels::Tx;

                let mut errs = Vec::new();
                #(
                    match self.#field_name.flush() {
                        Ok(()) => {},
                        Err(err) => errs.push((#field_index, err)),
                    }
                )*
                if errs.is_empty() {
                    Ok(())
                } else {
                    Err(nodo::channels::MultiFlushError(errs))
                }
            }

            fn check_connection(&self) -> nodo::channels::ConnectionCheck {
                use nodo::channels::Tx;

                let mut cc = nodo::channels::ConnectionCheck::new(#fields_count);
                #(cc.mark(#field_index, self.#field_name.is_connected());;)*
                cc
            }
        }
    };
    gen.into()
}
