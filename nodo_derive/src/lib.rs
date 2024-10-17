use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Fields, Meta};

/// Derive macro to implement the RxBundle trait for a custom struct with Rx fields
#[proc_macro_derive(RxBundleDerive)]
pub fn rx_bundle_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    impl_rx_bundle_derive(&input)
}

fn impl_rx_bundle_derive(input: &syn::DeriveInput) -> TokenStream {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
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
        impl #impl_generics nodo::channels::RxBundle for #name #type_generics #where_clause {
            fn len(&self) -> usize {
                #fields_count
            }

            fn name(&self, index: usize) -> String {
                match index {
                    #(#field_index => (#field_name_str).to_string(),)*
                    _ => panic!("invalid rx bundle index {index} for `{}`", #name_str),
                }
            }

            fn sync_all(&mut self, results: &mut [nodo::channels::SyncResult]) {
                use nodo::channels::Rx;

                #(results[#field_index] = self.#field_name.sync();)*
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
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
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
        impl #impl_generics nodo::channels::TxBundle for #name #type_generics #where_clause {
            fn len(&self) -> usize {
                #fields_count
            }

            fn name(&self, index: usize) -> String {
                match index {
                    #(#field_index => (#field_name_str).to_string(),)*
                    _ => panic!("invalid tx bundle index {index} for `{}`", #name_str),
                }
            }

            fn flush_all(&mut self, results: &mut [nodo::channels::FlushResult]) {
                use nodo::channels::Tx;

                #(results[#field_index] = self.#field_name.flush();)*
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

#[proc_macro_derive(Status, attributes(label, default, skipped))]
pub fn derive_status(input: TokenStream) -> TokenStream {
    // Parse the input token stream (the enum)
    let input = parse_macro_input!(input as DeriveInput);

    // Get the enum name
    let enum_name = input.ident.clone();

    // Ensure we have an enum
    let data = if let Data::Enum(DataEnum { variants, .. }) = input.data {
        variants
    } else {
        return syn::Error::new_spanned(input, "Status can only be derived for enums")
            .to_compile_error()
            .into();
    };

    let mut default_variant = None;
    let mut match_arms_status = Vec::new();
    let mut match_arms_label = Vec::new();

    // Iterate over each variant
    for variant in data {
        let variant_name = &variant.ident;
        let mut label = None;
        let mut is_default = false;
        let mut is_skipped = false;

        // Parse the attributes on each variant
        for attr in variant.attrs {
            if attr.path.is_ident("label") {
                if let Ok(Meta::NameValue(meta_name_value)) = attr.parse_meta() {
                    if let syn::Lit::Str(lit_str) = &meta_name_value.lit {
                        label = Some(lit_str.value());
                    }
                }
            } else if attr.path.is_ident("default") {
                is_default = true;
            } else if attr.path.is_ident("skipped") {
                is_skipped = true;
            }
        }

        // Handle different variant types (unit, tuple, and struct)
        let pattern = match &variant.fields {
            Fields::Unit => quote! { #enum_name::#variant_name },
            Fields::Unnamed(_) => quote! { #enum_name::#variant_name(..) },
            Fields::Named(_) => quote! { #enum_name::#variant_name { .. } },
        };

        // Generate match arms for as_default_status
        let default_status = if is_skipped {
            quote! { DefaultStatus::Skipped }
        } else {
            quote! { DefaultStatus::Running }
        };
        match_arms_status.push(quote! {
            #pattern => #default_status,
        });

        // Generate match arms for label, defaulting to the variant's name if no label is provided
        let label = label.unwrap_or_else(|| variant_name.to_string());
        match_arms_label.push(quote! {
            #pattern => #label,
        });

        // Set the default variant
        if is_default {
            default_variant = Some(quote! {
                fn default_implementation_status() -> Self {
                    #enum_name::#variant_name
                }
            });
        }
    }

    // Generate the default implementation status function
    let default_implementation_status = default_variant.unwrap_or_else(|| {
        quote! {
            fn default_implementation_status() -> Self {
                panic!("No default status was specified for the enum");
            }
        }
    });

    // Generate the final implementation
    let expanded = quote! {
        impl CodeletStatus for #enum_name {
            #default_implementation_status

            fn as_default_status(&self) -> DefaultStatus {
                match self {
                    #(#match_arms_status)*
                }
            }

            fn label(&self) -> &str {
                match self {
                    #(#match_arms_label)*
                }
            }
        }
    };

    // Convert the generated code into a token stream
    TokenStream::from(expanded)
}
