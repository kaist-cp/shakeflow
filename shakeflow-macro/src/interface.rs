use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

use super::utils::*;

pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let vis = &ast.vis;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    let fname = format!("{}Fwd", name);
    let fident = syn::Ident::new(&fname, name.span());
    let bname = format!("{}Bwd", name);
    let bident = syn::Ident::new(&bname, name.span());
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        todo!()
    };

    let names = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap();
        quote! { #name }
    });

    // fields for forward value.
    let fwd_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! { #vis #name: <#ty as Interface>::Fwd }
    });

    // fields for backward value.
    let bwd_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! { #vis #name: <#ty as Interface>::Bwd }
    });

    // fields for `interface_typ`.
    let interface_typ_fields = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        let symbol = get_member_symbol(&f.attrs, name).unwrap();
        let sep = get_member_sep(&f.attrs);

        match sep {
            None => quote! { _inner.insert(#symbol.to_string(), (None, <#ty>::interface_typ())) },
            Some(sep) => {
                quote! { _inner.insert(#symbol.to_string(), (Some(#sep.to_string()), <#ty>::interface_typ())) }
            }
        }
    });

    // fields for `try_from_inner`.
    let try_from_inner_fields = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap();
        let ty = &f.ty;
        let symbol = get_member_symbol(&f.attrs, name).unwrap();
        quote! { let #name = <#ty>::try_from_inner(_inner.remove(#symbol).unwrap().1)? }
    });

    // fields for `try_into_inner`.
    let try_into_inner_fields = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap();
        let symbol = get_member_symbol(&f.attrs, name).unwrap();
        let sep = get_member_sep(&f.attrs);

        match sep {
            None => quote! { _inner.insert(#symbol.to_string(), (None, self.#name.try_into_inner()?)) },
            Some(sep) => {
                quote! { _inner.insert(#symbol.to_string(), (Some(#sep.to_string()), self.#name.try_into_inner()?)) }
            }
        }
    });

    let expanded = quote! {
        #[allow(unused_braces, missing_docs)]
        #[derive(Debug, Clone, Signal)]
        #vis struct #fident #impl_generics #where_clause {
            #(#fwd_fields,)*
        }
        #[allow(unused_braces, missing_docs)]
        #[derive(Debug, Clone, Signal)]
        #vis struct #bident #impl_generics #where_clause {
            #(#bwd_fields,)*
        }
        #[allow(unused_braces, missing_docs)]
        impl #impl_generics Interface for #name #ty_generics #where_clause {
            type Fwd = #fident #ty_generics;
            type Bwd = #bident #ty_generics;
            fn interface_typ() -> lir::InterfaceTyp {
                let mut _inner = ::shakeflow::linked_hash_map::LinkedHashMap::new();
                #(#interface_typ_fields;)*
                lir::InterfaceTyp::Struct(_inner)
            }
            fn try_from_inner(interface: lir::Interface) -> Result<Self, shakeflow::hir::InterfaceError> {
                match interface {
                    lir::Interface::Struct(mut _inner) => {
                        #(#try_from_inner_fields;)*
                        assert!(_inner.is_empty(), "internal compiler error");
                        Ok(Self { #(#names,)* })
                    }
                    _ => todo!(),
                }
            }
            fn try_into_inner(self) -> Result<lir::Interface, shakeflow::hir::InterfaceError> {
                let mut _inner = ::shakeflow::linked_hash_map::LinkedHashMap::new();
                #(#try_into_inner_fields;)*
                Ok(lir::Interface::Struct(_inner))
            }
        }
    };
    expanded.into()
}
