use std::borrow::Cow;
use std::str::FromStr;

use proc_macro::Span;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro_derive(Init)]
pub fn derive_init(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let syn::Data::Struct(data_struct) = input.data else {
        return proc_macro::TokenStream::from_str(r#"compile_error!("TODO");"#).unwrap();
    };

    let field_count = data_struct.fields.len();

    let initailizer_name: syn::Ident =
        syn::Ident::new(&format!("{}Init", input.ident), Span::call_site().into());
    let generic_names: Vec<syn::Ident> = (0..field_count)
        .map(|n| syn::Ident::new(&format!("_Field{n}"), Span::call_site().into()))
        .collect();
    let generics: Vec<syn::GenericParam> = generic_names
        .iter()
        .map(|name| syn::GenericParam::Type(syn::TypeParam::from(name.clone())))
        .collect();

    //TODO: use quote instead

    let initializer_struct = syn::ItemStruct {
        attrs: vec![],
        vis: input.vis,
        struct_token: data_struct.struct_token,
        ident: initailizer_name.clone(),
        generics: syn::Generics {
            lt_token: None,
            params: syn::punctuated::Punctuated::from_iter(generics.iter().cloned()),
            gt_token: None,
            where_clause: None,
        },
        fields: syn::Fields::Unnamed(syn::FieldsUnnamed {
            paren_token: Default::default(),
            unnamed: syn::punctuated::Punctuated::from_iter(generic_names.iter().map(|tn| {
                syn::Field {
                    attrs: vec![],
                    vis: syn::Visibility::Public(Default::default()),
                    mutability: syn::FieldMutability::None,
                    ident: None,
                    colon_token: None,
                    ty: syn::Type::Path(syn::TypePath {
                        qself: None,
                        path: syn::Path {
                            leading_colon: None,
                            segments: syn::punctuated::Punctuated::from_iter([
                                syn::PathSegment::from(tn.clone()),
                            ]),
                        },
                    }),
                }
            })),
        }),
        semi_token: Default::default(),
    };

    let struct_name = input.ident;
    let (_, struct_generics, where_clause) = input.generics.split_for_impl();
    let impl_generics = &input.generics.params;
    let replace_self_with: syn::Type = syn::parse_quote!( #struct_name #struct_generics );
    // Fix Self usages to be the correct name
    struct FixSelfVisitor {
        replace_self_with: syn::Type,
    }

    impl syn::visit_mut::VisitMut for FixSelfVisitor {
        fn visit_type_mut(&mut self, node: &mut syn::Type) {
            if let syn::Type::Path(i) = node
                && i.qself.is_none()
                && let Some(first) = i.path.segments.first()
                && first.ident == "Self"
            {
                if i.path.segments.len() > 1 {
                    // Replace first path segment with qualified self if this is a path
                    let new_segments = i.path.segments.iter().skip(1).cloned().collect();
                    i.path.leading_colon = Some(Default::default());
                    i.qself = Some(syn::QSelf {
                        lt_token: Default::default(),
                        ty: Box::new(self.replace_self_with.clone()),
                        position: 0,
                        as_token: None,
                        gt_token: Default::default(),
                    });
                    i.path.segments = new_segments;
                } else {
                    // Else just replace the whole type
                    *node = self.replace_self_with.clone();
                }
            }

            syn::visit_mut::visit_type_mut(self, node);
        }
    }

    let mut visitor = FixSelfVisitor { replace_self_with };
    let field_tys: Vec<_> = data_struct
        .fields
        .iter()
        .map(|f| f.ty.clone())
        .map(|mut ty| {
            use syn::visit_mut::VisitMut;
            visitor.visit_type_mut(&mut ty);
            ty
        })
        .collect();

    let tail_ty: Vec<_> = field_tys.last().into_iter().cloned().collect();
    let tail_generic: Vec<_> = generic_names.last().into_iter().cloned().collect();

    let (extra_generic, extra_type): (TokenStream2, TokenStream2) = if field_count == 0 {
        (TokenStream2::new(), "()".parse().unwrap())
    } else if field_count == 1 {
        ("Extra,".parse().unwrap(), "Extra".parse().unwrap())
    } else {
        ("Extra: Clone,".parse().unwrap(), "Extra".parse().unwrap())
    };

    let dst_members = data_struct.fields.members();

    let where_clause = match where_clause {
        Some(wc) if !wc.predicates.is_empty() => {
            let mut wc = Cow::Borrowed(wc);
            if !wc.predicates.trailing_punct() {
                wc.to_mut().predicates.push_punct(Default::default());
            }
            Box::new(wc) as Box<dyn quote::ToTokens>
        }
        _ => Box::new(TokenStream2::from_str("where").unwrap()),
    };

    quote::quote! {
        #initializer_struct

        unsafe impl<#extra_generic __Error, #( #generic_names, )* #impl_generics > ::in_place_init::PinInit<#struct_name #struct_generics, __Error, #extra_type> for #initailizer_name< #(#generic_names,)* >
            #where_clause
                #( #generic_names: ::in_place_init::PinInit<#field_tys, __Error, #extra_type>, )*
        {

            fn metadata(&self) #( -> <#tail_ty as ::core::ptr::Pointee>::Metadata )* {
                let Self( #( #generic_names, )* ) = self;
                #( #tail_generic.metadata() )*
            }

            unsafe fn init(self, dst: *mut #struct_name #struct_generics, extra: #extra_type) -> Result<(), __Error> {
                let Self(#(#generic_names,)*) = self;
                #(
                    let #generic_names = {
                        let dst = &raw mut (*dst).#dst_members;
                        #generic_names.init(dst, extra.clone())?;
                        ::in_place_init::noop_allocator::owning_ref::from_raw(dst)
                    };
                )*
                ::core::mem::forget((#(#generic_names,)*));
                Ok(())
            }
        }

        unsafe impl<#extra_generic __Error, #( #generic_names, )* #impl_generics > ::in_place_init::Init<#struct_name #struct_generics, __Error, #extra_type> for #initailizer_name< #(#generic_names,)* >
            where
                #( #generic_names: ::in_place_init::Init<#field_tys, __Error, #extra_type>, )*
        {}

    }.into()
}
