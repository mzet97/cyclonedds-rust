use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DeriveInput, Expr, ExprLit, Field, Fields, GenericArgument, Lit,
    PathArguments, Type, TypePath,
};

type PrimitiveArrayInfo = (TokenStream2, TokenStream2, Vec<TokenStream2>, u32);
type DirectVecInfo = (TokenStream2, u32, TokenStream2);

#[proc_macro_derive(DdsType, attributes(key, dds_enum))]
pub fn derive_dds_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(DdsEnum)]
pub fn derive_dds_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_enum_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(DdsUnion, attributes(dds_discriminant, dds_case, dds_default))]
pub fn derive_dds_union(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_union_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[proc_macro_derive(DdsBitmask, attributes(bit_bound))]
pub fn derive_dds_bitmask(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_bitmask_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_impl(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let type_name_str = name.to_string();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(f) => &f.named,
            Fields::Unnamed(_) => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "DdsType only supports structs with named fields",
                ))
            }
            Fields::Unit => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "DdsType cannot be derived for unit structs",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "DdsType can only be derived for structs",
            ))
        }
    };

    let mut main_ops_parts: Vec<TokenStream2> = Vec::new();
    let mut tail_block_parts: Vec<TokenStream2> = Vec::new();
    let mut key_steps: Vec<TokenStream2> = Vec::new();
    let mut post_key_steps: Vec<TokenStream2> = Vec::new();
    let mut clone_fields: Vec<TokenStream2> = Vec::new();
    let mut native_fields: Vec<TokenStream2> = Vec::new();
    let mut native_init_fields: Vec<TokenStream2> = Vec::new();
    let mut len_exprs: Vec<TokenStream2> = Vec::new();
    let mut key_count_exprs: Vec<TokenStream2> = Vec::new();
    let mut uses_native = false;
    let native_name = format_ident!("__CycloneDdsNative{}", name);

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty = &field.ty;
        let is_key = field.attrs.iter().any(|a| {
            a.path()
                .segments
                .last()
                .map(|s| s.ident == "key")
                .unwrap_or(false)
        });
        let is_enum = field.attrs.iter().any(|a| {
            a.path()
                .segments
                .last()
                .map(|s| s.ident == "dds_enum")
                .unwrap_or(false)
        });

        let offset_expr = quote! {
            ::std::mem::offset_of!(#native_name, #field_name) as u32
        };

        let primitive_type = field_typecode(field)?;
        let option_inner = option_inner_type(field_ty)?;
        let direct_string = is_direct_string(field_ty);
        let direct_vec = direct_vec_info(field_ty)?;
        let direct_vec_string = is_direct_vec_string(field_ty)?;
        let direct_vec_composite = direct_vec_composite_info(field_ty)?;
        let composite_seq = composite_sequence_type(field_ty)?;
        let composite_bounded_seq = composite_bounded_sequence_type(field_ty)?;
        let nested_sequence = nested_sequence_info(field_ty, &offset_expr)?;
        let bounded_type = bounded_sequence_typecode(field_ty)?;
        let primitive_array = primitive_or_string_array_info(field_ty)?;
        let composite_arr = composite_array_type(field_ty)?;
        let enum_seq = if is_enum {
            enum_sequence_info(field_ty)?
        } else {
            None
        };
        let enum_bounded_seq = if is_enum {
            enum_bounded_sequence_info(field_ty)?
        } else {
            None
        };
        let enum_array = if is_enum {
            enum_array_info(field_ty)?
        } else {
            None
        };
        let enum_direct_vec = if is_enum {
            enum_direct_vec_info(field_ty)?
        } else {
            None
        };
        let is_composite_nested = primitive_type.is_none()
            && option_inner.is_none()
            && composite_seq.is_none()
            && composite_bounded_seq.is_none()
            && nested_sequence.is_none()
            && bounded_type.is_none()
            && direct_vec.is_none()
            && direct_vec_string.is_none()
            && direct_vec_composite.is_none()
            && !direct_string
            && primitive_array.is_none()
            && composite_arr.is_none()
            && enum_seq.is_none()
            && enum_bounded_seq.is_none()
            && enum_array.is_none()
            && enum_direct_vec.is_none()
            && !is_enum;

        let len_expr = if let Some(inner_ty) = option_inner.clone() {
            uses_native = true;
            if is_key {
                return Err(syn::Error::new_spanned(
                    field,
                    "optional keyed fields are not supported yet",
                ));
            }
            if is_enum {
                main_ops_parts.push(quote! {
                    __ops.push(
                        cyclonedds::OP_ADR
                            | cyclonedds::OP_FLAG_OPT
                            | cyclonedds::OP_FLAG_EXT
                            | cyclonedds::TYPE_ENU
                            | <#inner_ty as cyclonedds::DdsEnumType>::enum_op_flags()
                    );
                    __ops.push(#offset_expr);
                    __ops.push(<#inner_ty as cyclonedds::DdsEnumType>::max_discriminant());
                });
                quote! { 3u32 }
            } else if is_direct_string(&inner_ty) {
                main_ops_parts.push(quote! {
                    __ops.push(cyclonedds::OP_ADR | cyclonedds::OP_FLAG_OPT | cyclonedds::OP_FLAG_EXT | cyclonedds::TYPE_STR);
                    __ops.push(#offset_expr);
                });
                quote! { 2u32 }
            } else if let Some((inner_typecode, word_count)) = field_typecode_from_type(&inner_ty)?
            {
                main_ops_parts.push(quote! {
                    __ops.push(cyclonedds::OP_ADR | cyclonedds::OP_FLAG_OPT | cyclonedds::OP_FLAG_EXT | #inner_typecode);
                    __ops.push(#offset_expr);
                });
                quote! { #word_count }
            } else {
                main_ops_parts.push(quote! {
                    __ops.push(cyclonedds::OP_ADR | cyclonedds::OP_FLAG_OPT | cyclonedds::OP_FLAG_EXT | cyclonedds::TYPE_EXT);
                    __ops.push(#offset_expr);
                    __ops.push(0u32);
                    __ops.push(::std::mem::size_of::<#inner_ty>() as u32);
                });
                tail_block_parts.push(quote! {
                    __tail_blocks.push({
                        let mut __v = ::std::vec::Vec::new();
                        __v.extend(<#inner_ty as cyclonedds::DdsType>::ops());
                        __v.push(cyclonedds::OP_RTS);
                        __v
                    });
                });
                quote! { 4u32 + <#inner_ty as cyclonedds::DdsType>::ops().len() as u32 + 1u32 }
            }
        } else if is_enum {
            if let Some(inner_ty) = enum_direct_vec.clone() {
                if is_key {
                    return Err(syn::Error::new_spanned(
                        field,
                        "keyed Vec<Enum> fields are not supported yet",
                    ));
                }
                uses_native = true;
                main_ops_parts.push(quote! {
                    __ops.push(
                        cyclonedds::OP_ADR
                            | cyclonedds::TYPE_SEQ
                            | cyclonedds::SUBTYPE_ENU
                            | <#inner_ty as cyclonedds::DdsEnumType>::enum_op_flags()
                    );
                    __ops.push(#offset_expr);
                    __ops.push(<#inner_ty as cyclonedds::DdsEnumType>::max_discriminant());
                });
                quote! { 3u32 }
            } else if let Some(inner_ty) = enum_seq {
                if is_key {
                    return Err(syn::Error::new_spanned(
                        field,
                        "keyed sequence-of-enum fields are not supported yet",
                    ));
                }
                main_ops_parts.push(quote! {
                    __ops.push(
                        cyclonedds::OP_ADR
                            | cyclonedds::TYPE_SEQ
                            | cyclonedds::SUBTYPE_ENU
                            | <#inner_ty as cyclonedds::DdsEnumType>::enum_op_flags()
                    );
                    __ops.push(#offset_expr);
                    __ops.push(<#inner_ty as cyclonedds::DdsEnumType>::max_discriminant());
                });
                quote! { 3u32 }
            } else if let Some((inner_ty, bound_expr)) = enum_bounded_seq {
                if is_key {
                    return Err(syn::Error::new_spanned(
                        field,
                        "keyed bounded-sequence-of-enum fields are not supported yet",
                    ));
                }
                main_ops_parts.push(quote! {
                    __ops.push(
                        cyclonedds::OP_ADR
                            | cyclonedds::TYPE_BSQ
                            | cyclonedds::SUBTYPE_ENU
                            | <#inner_ty as cyclonedds::DdsEnumType>::enum_op_flags()
                    );
                    __ops.push(#offset_expr);
                    __ops.push(#bound_expr);
                    __ops.push(<#inner_ty as cyclonedds::DdsEnumType>::max_discriminant());
                });
                quote! { 4u32 }
            } else if let Some((len_expr_tokens, flags_expr, max_expr)) = enum_array {
                if is_key {
                    return Err(syn::Error::new_spanned(
                        field,
                        "keyed array-of-enum fields are not supported yet",
                    ));
                }
                main_ops_parts.push(quote! {
                    __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_ENU | #flags_expr);
                    __ops.push(#offset_expr);
                    __ops.push(#len_expr_tokens);
                    __ops.push(#max_expr);
                });
                quote! { 4u32 }
            } else if is_key {
                main_ops_parts.push(quote! {
                    __ops.push(cyclonedds::OP_ADR | cyclonedds::OP_FLAG_KEY | cyclonedds::TYPE_ENU | <#field_ty as cyclonedds::DdsEnumType>::enum_op_flags());
                    __ops.push(#offset_expr);
                    __ops.push(<#field_ty as cyclonedds::DdsEnumType>::max_discriminant());
                });
                quote! { 3u32 }
            } else {
                main_ops_parts.push(quote! {
                    __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_ENU | <#field_ty as cyclonedds::DdsEnumType>::enum_op_flags());
                    __ops.push(#offset_expr);
                    __ops.push(<#field_ty as cyclonedds::DdsEnumType>::max_discriminant());
                });
                quote! { 3u32 }
            }
        } else if let Some((typecode_expr, word_count, _native_ty)) = direct_vec.clone() {
            uses_native = true;
            main_ops_parts.push(quote! {
                __ops.push(cyclonedds::OP_ADR | #typecode_expr);
                __ops.push(#offset_expr);
            });
            quote! { #word_count }
        } else if let Some(inner_ty) = direct_vec_composite.clone() {
            uses_native = true;
            main_ops_parts.push(quote! {
                __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_STU);
                __ops.push(#offset_expr);
                __ops.push(::std::mem::size_of::<#inner_ty>() as u32);
                __ops.push((4u32 << 16) + 5u32);
                __ops.push(cyclonedds::OP_RTS);
                __ops.extend(<#inner_ty as cyclonedds::DdsType>::ops());
                __ops.push(cyclonedds::OP_RTS);
            });
            quote! { 6u32 + <#inner_ty as cyclonedds::DdsType>::ops().len() as u32 }
        } else if direct_vec_string.is_some() {
            uses_native = true;
            main_ops_parts.push(quote! {
                __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_STR);
                __ops.push(#offset_expr);
            });
            quote! { 2u32 }
        } else if direct_string {
            uses_native = true;
            main_ops_parts.push(quote! {
                __ops.extend(cyclonedds::adr(cyclonedds::TYPE_STR, #offset_expr));
            });
            quote! { 2u32 }
        } else if let Some((child_block, total_len)) = nested_sequence {
            if is_key {
                return Err(syn::Error::new_spanned(
                    field,
                    "keyed nested sequence fields are not supported yet",
                ));
            }
            main_ops_parts.push(child_block);
            total_len
        } else if let Some(inner_ty) = composite_seq {
            if is_key {
                return Err(syn::Error::new_spanned(
                    field,
                    "keyed sequence-of-struct fields are not supported yet",
                ));
            }
            main_ops_parts.push(quote! {
                __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_STU);
                __ops.push(#offset_expr);
                __ops.push(::std::mem::size_of::<#inner_ty>() as u32);
                __ops.push((4u32 << 16) + 5u32);
                __ops.push(cyclonedds::OP_RTS);
                __ops.extend(<#inner_ty as cyclonedds::DdsType>::ops());
                __ops.push(cyclonedds::OP_RTS);
            });
            quote! { 6u32 + <#inner_ty as cyclonedds::DdsType>::ops().len() as u32 }
        } else if let Some((inner_ty, bound_expr)) = composite_bounded_seq {
            if is_key {
                return Err(syn::Error::new_spanned(
                    field,
                    "keyed bounded-sequence-of-struct fields are not supported yet",
                ));
            }
            main_ops_parts.push(quote! {
                __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_STU);
                __ops.push(#offset_expr);
                __ops.push(#bound_expr);
                __ops.push(::std::mem::size_of::<#inner_ty>() as u32);
                __ops.push((5u32 << 16) + 6u32);
                __ops.push(cyclonedds::OP_RTS);
                __ops.extend(<#inner_ty as cyclonedds::DdsType>::ops());
                __ops.push(cyclonedds::OP_RTS);
            });
            quote! { 7u32 + <#inner_ty as cyclonedds::DdsType>::ops().len() as u32 }
        } else if let Some((typecode_expr, bound_expr)) = bounded_type {
            uses_native = true;
            if is_key {
                main_ops_parts.push(quote! {
                    __ops.push(cyclonedds::OP_ADR | cyclonedds::OP_FLAG_KEY | #typecode_expr);
                    __ops.push(#offset_expr);
                    __ops.push(#bound_expr);
                });
            } else {
                main_ops_parts.push(quote! {
                    __ops.push(cyclonedds::OP_ADR | #typecode_expr);
                    __ops.push(#offset_expr);
                    __ops.push(#bound_expr);
                });
            }
            quote! { 3u32 }
        } else if let Some((typecode_expr, len_expr_tokens, extra_exprs, word_count)) =
            primitive_array
        {
            if is_key {
                return Err(syn::Error::new_spanned(
                    field,
                    "keyed primitive/string arrays are not supported yet",
                ));
            }
            main_ops_parts.push(quote! {
                __ops.push(cyclonedds::OP_ADR | #typecode_expr);
                __ops.push(#offset_expr);
                __ops.push(#len_expr_tokens);
                #(__ops.push(#extra_exprs);)*
            });
            quote! { #word_count }
        } else if let Some(inner_ty) = composite_arr {
            if is_key {
                return Err(syn::Error::new_spanned(
                    field,
                    "keyed array-of-struct fields are not supported yet",
                ));
            }
            let Type::Array(arr) = field_ty else {
                unreachable!()
            };
            let len_expr_tokens = &arr.len;
            main_ops_parts.push(quote! {
                __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_STU);
                __ops.push(#offset_expr);
                __ops.push((#len_expr_tokens) as u32);
                __ops.push((5u32 << 16) + 6u32);
                __ops.push(::std::mem::size_of::<#inner_ty>() as u32);
                __ops.push(cyclonedds::OP_RTS);
                __ops.extend(<#inner_ty as cyclonedds::DdsType>::ops());
                __ops.push(cyclonedds::OP_RTS);
            });
            quote! { 7u32 + <#inner_ty as cyclonedds::DdsType>::ops().len() as u32 }
        } else if let Some((typecode_expr, word_count)) = primitive_type {
            // DdsSequence / DdsBoundedSequence fields need native struct conversion
            // because the CDR serializer uses offsets from the #[repr(C)] native layout.
            if sequence_typecode(field_ty)?.is_some() {
                uses_native = true;
            }
            if is_key {
                main_ops_parts.push(quote! {
                    __ops.extend(cyclonedds::adr_key(#typecode_expr, #offset_expr));
                });
            } else {
                main_ops_parts.push(quote! {
                    __ops.extend(cyclonedds::adr(#typecode_expr, #offset_expr));
                });
            }
            quote! { #word_count }
        } else {
            main_ops_parts.push(quote! {
                __ops.push(
                    cyclonedds::OP_ADR
                        | if <#field_ty as cyclonedds::DdsType>::key_count() > 0 {
                            cyclonedds::OP_FLAG_KEY
                        } else {
                            0
                        }
                        | cyclonedds::TYPE_EXT
                );
                __ops.push(#offset_expr);
                __ops.push(0);
            });
            tail_block_parts.push(quote! {
                __tail_blocks.push({
                    let mut __v = ::std::vec::Vec::new();
                    __v.extend(<#field_ty as cyclonedds::DdsType>::ops());
                    __v.push(cyclonedds::OP_RTS);
                    __v
                });
            });
            quote! { 3u32 + <#field_ty as cyclonedds::DdsType>::ops().len() as u32 + 1u32 }
        };
        key_steps.push(if is_key {
            quote! {
                __keys.push(cyclonedds::KeyDescriptor { name: stringify!(#field_name).into(), ops_path: vec![__ops_index] });
            }
        } else if is_composite_nested
        {
            quote! {
                for __child_key in <#field_ty as cyclonedds::DdsType>::keys() {
                    let mut __ops_path = ::std::vec::Vec::with_capacity(1 + __child_key.ops_path.len());
                    __ops_path.push(__ops_index);
                    __ops_path.extend(__child_key.ops_path.iter().copied());
                    __keys.push(cyclonedds::KeyDescriptor {
                        name: format!("{}.{}", stringify!(#field_name), __child_key.name),
                        ops_path: __ops_path,
                    });
                }
            }
        } else {
            quote! {}
        });
        post_key_steps.push(if option_inner.is_some() {
            quote! {
                __post.push(cyclonedds::OP_MID | __ops_index);
                __post.push(__member_id);
                __member_id += 1;
            }
        } else {
            quote! {}
        });

        clone_fields.push(quote! {
            #field_name: ::std::clone::Clone::clone(&__raw.#field_name),
        });
        if let Some(inner_ty) = option_inner.as_ref() {
            native_fields.push(quote! { #field_name: *mut ::std::ffi::c_void, });
            native_init_fields.push(if is_enum {
                quote! {
                    #field_name: if let Some(value) = &self.#field_name {
                        arena.hold(::std::clone::Clone::clone(value)) as *const #inner_ty as *mut ::std::ffi::c_void
                    } else {
                        ::std::ptr::null_mut()
                    },
                }
            } else if is_direct_string(inner_ty) {
                quote! {
                    #field_name: if let Some(value) = &self.#field_name {
                        arena.hold(cyclonedds::DdsString::new(value)?) as *const cyclonedds::DdsString as *mut ::std::ffi::c_void
                    } else {
                        ::std::ptr::null_mut()
                    },
                }
            } else if field_typecode_from_type(inner_ty)?.is_some() {
                quote! {
                    #field_name: if let Some(value) = &self.#field_name {
                        arena.hold(::std::clone::Clone::clone(value)) as *const #inner_ty as *mut ::std::ffi::c_void
                    } else {
                        ::std::ptr::null_mut()
                    },
                }
            } else {
                quote! {
                    #field_name: if let Some(value) = &self.#field_name {
                        value.write_to_native(arena)? as *mut ::std::ffi::c_void
                    } else {
                        ::std::ptr::null_mut()
                    },
                }
            });
            clone_fields.pop();
            clone_fields.push(if is_enum {
                quote! {
                    #field_name: if __raw.#field_name.is_null() {
                        None
                    } else {
                        Some(::std::ptr::read(__raw.#field_name as *const #inner_ty))
                    },
                }
            } else if is_direct_string(inner_ty) {
                quote! {
                    #field_name: if __raw.#field_name.is_null() {
                        None
                    } else {
                        Some((*( __raw.#field_name as *const cyclonedds::DdsString)).to_string_lossy())
                    },
                }
            } else if field_typecode_from_type(inner_ty)?.is_some() {
                quote! {
                    #field_name: if __raw.#field_name.is_null() {
                        None
                    } else {
                        Some(::std::ptr::read(__raw.#field_name as *const #inner_ty))
                    },
                }
            } else {
                quote! {
                    #field_name: if __raw.#field_name.is_null() {
                        None
                    } else {
                        Some(<#inner_ty as cyclonedds::DdsType>::clone_out(__raw.#field_name as *const #inner_ty))
                    },
                }
            });
        } else if direct_string {
            native_fields.push(quote! { #field_name: cyclonedds::DdsString, });
            native_init_fields.push(quote! {
                #field_name: cyclonedds::DdsString::new(&self.#field_name)?,
            });
            clone_fields.pop();
            clone_fields.push(quote! {
                #field_name: __raw.#field_name.to_string_lossy(),
            });
        } else if let Some((_, _, native_ty)) = direct_vec.as_ref() {
            native_fields.push(quote! { #field_name: #native_ty, });
            native_init_fields.push(quote! {
                #field_name: <#native_ty>::from_slice(&self.#field_name)?,
            });
            clone_fields.pop();
            clone_fields.push(quote! {
                #field_name: __raw.#field_name.to_vec(),
            });
        } else if let Some(inner_ty) = direct_vec_composite.as_ref() {
            let native_ty = quote! { cyclonedds::DdsSequence<#inner_ty> };
            native_fields.push(quote! { #field_name: #native_ty, });
            native_init_fields.push(quote! {
                #field_name: <#native_ty>::from_slice(&self.#field_name)?,
            });
            clone_fields.pop();
            clone_fields.push(quote! {
                #field_name: __raw.#field_name.to_vec(),
            });
        } else if direct_vec_string.is_some() {
            native_fields
                .push(quote! { #field_name: cyclonedds::DdsSequence<cyclonedds::DdsString>, });
            native_init_fields.push(quote! {
                #field_name: {
                    let __converted: ::std::vec::Vec<cyclonedds::DdsString> = self
                        .#field_name
                        .iter()
                        .map(|value| cyclonedds::DdsString::new(value))
                        .collect::<cyclonedds::DdsResult<_>>()?;
                    cyclonedds::DdsSequence::<cyclonedds::DdsString>::from_slice(&__converted)?
                },
            });
            clone_fields.pop();
            clone_fields.push(quote! {
                #field_name: __raw
                    .#field_name
                    .to_vec()
                    .into_iter()
                    .map(|value| value.to_string_lossy())
                    .collect(),
            });
        } else if let Some(inner_ty) = enum_direct_vec.as_ref() {
            let native_ty = quote! { cyclonedds::DdsSequence<#inner_ty> };
            native_fields.push(quote! { #field_name: #native_ty, });
            native_init_fields.push(quote! {
                #field_name: <#native_ty>::from_slice(&self.#field_name)?,
            });
            clone_fields.pop();
            clone_fields.push(quote! {
                #field_name: __raw.#field_name.to_vec(),
            });
        } else {
            native_fields.push(quote! { #field_name: #field_ty, });
            native_init_fields.push(quote! {
                #field_name: ::std::clone::Clone::clone(&self.#field_name),
            });
        }
        len_exprs.push(len_expr);
        key_count_exprs.push(if is_key {
            quote! { 1usize }
        } else if is_composite_nested {
            quote! { <#field_ty as cyclonedds::DdsType>::key_count() }
        } else {
            quote! { 0usize }
        });
    }

    let native_struct = quote! {
        #[repr(C)]
        struct #native_name {
            #(#native_fields)*
        }
    };

    let descriptor_methods = if uses_native {
        quote! {
            fn write_to_native<'a>(
                &'a self,
                arena: &'a mut cyclonedds::write_arena::WriteArena,
            ) -> cyclonedds::DdsResult<*const ::std::ffi::c_void> {
                let native = #native_name {
                    #(#native_init_fields)*
                };
                Ok(arena.hold(native) as *const #native_name as *const ::std::ffi::c_void)
            }
        }
    } else {
        quote! {}
    };

    let clone_ptr = quote! {
        let __raw = &*(ptr as *const #native_name);
    };

    let expanded = quote! {
        #native_struct

        impl cyclonedds::DdsType for #name {
            fn type_name() -> &'static str { #type_name_str }

            fn descriptor_size() -> u32 {
                ::std::mem::size_of::<#native_name>() as u32
            }

            fn descriptor_align() -> u32 {
                ::std::mem::align_of::<#native_name>() as u32
            }

            fn ops() -> ::std::vec::Vec<u32> {
                let mut __ops = ::std::vec::Vec::new();
                let mut __tail_blocks: ::std::vec::Vec<::std::vec::Vec<u32>> = ::std::vec::Vec::new();
                #(#main_ops_parts)*
                #(#tail_block_parts)*
                let mut __patch_positions = ::std::vec::Vec::new();
                let mut __scan = 0usize;
                while __scan < __ops.len() {
                    let __op = __ops[__scan];
                    if (__op & cyclonedds::DDS_OP_MASK_CONST) == cyclonedds::OP_ADR
                        && (__op & cyclonedds::DDS_OP_TYPE_MASK_CONST) == cyclonedds::TYPE_EXT
                    {
                        __patch_positions.push(__scan);
                        __scan += 3;
                    } else if (__op & cyclonedds::DDS_OP_MASK_CONST) == cyclonedds::OP_ADR {
                        let __primary = __op & cyclonedds::DDS_OP_TYPE_MASK_CONST;
                        let __subtype = __op & cyclonedds::DDS_OP_SUBTYPE_MASK_CONST;
                        __scan += match __primary {
                            cyclonedds::TYPE_BST => 3,
                            cyclonedds::TYPE_SEQ => if __subtype == cyclonedds::SUBTYPE_BST || __subtype == cyclonedds::SUBTYPE_STU { 4 } else { 2 },
                            cyclonedds::TYPE_BSQ => if __subtype == cyclonedds::SUBTYPE_BST || __subtype == cyclonedds::SUBTYPE_STU { 5 } else { 3 },
                            cyclonedds::TYPE_ARR => if __subtype == cyclonedds::SUBTYPE_STU { 5 } else if __subtype == cyclonedds::SUBTYPE_BST { 5 } else { 3 },
                            cyclonedds::TYPE_ENU => 3,
                            _ => 2,
                        };
                    } else {
                        __scan += 1;
                    }
                }
                __ops.push(cyclonedds::OP_RTS);
                let mut __tail_index = 0usize;
                for __patch_pos in __patch_positions {
                    let __child_start = __ops.len() as u32;
                    let __op = __ops[__patch_pos];
                    let __next_insn_words = if (__op & cyclonedds::OP_FLAG_EXT) != 0 {
                        4u32
                    } else {
                        3u32
                    };
                    __ops[__patch_pos + 2] =
                        (__next_insn_words << 16) + (__child_start - (__patch_pos as u32));
                    let __child_ops = __tail_blocks[__tail_index].clone();
                    __ops.extend(__child_ops);
                    __tail_index += 1;
                }
                __ops
            }

            unsafe fn clone_out(ptr: *const Self) -> Self {
                #clone_ptr
                Self {
                    #(#clone_fields)*
                }
            }

            #descriptor_methods

            fn key_count() -> usize { 0usize #(+ #key_count_exprs)* }

            fn keys() -> ::std::vec::Vec<cyclonedds::KeyDescriptor> {
                let mut __keys = ::std::vec::Vec::new();
                let mut __ops_index = 0u32;
                #(
                    #key_steps
                    __ops_index += #len_exprs;
                )*
                __keys
            }

            fn post_key_ops() -> ::std::vec::Vec<u32> {
                let mut __post = ::std::vec::Vec::new();
                let mut __ops_index = 0u32;
                let mut __member_id = 1u32;
                #(
                    #post_key_steps
                    __ops_index += #len_exprs;
                )*
                if !__post.is_empty() {
                    __post.push(cyclonedds::OP_RTS);
                }
                __post
            }
        }
    };

    Ok(expanded)
}

fn derive_enum_impl(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "DdsEnum can only be derived for enums",
            ))
        }
    };

    let mut next_value: i64 = 0;
    let mut max_value: u32 = 0;
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "DdsEnum only supports fieldless enums",
            ));
        }

        let value = if let Some((_, expr)) = &variant.discriminant {
            parse_int_literal(expr)?
        } else {
            next_value
        };
        if value < 0 {
            return Err(syn::Error::new_spanned(
                variant,
                "DdsEnum currently supports only non-negative discriminants",
            ));
        }
        max_value = max_value.max(value as u32);
        next_value = value + 1;
    }

    Ok(quote! {
        impl cyclonedds::DdsEnumType for #name {
            fn max_discriminant() -> u32 {
                #max_value
            }
        }
    })
}

// ---------------------------------------------------------------------------
// DdsUnion derive
// ---------------------------------------------------------------------------

/// Supported discriminant types and their CycloneDDS typecodes.
fn discriminant_info(ty: &Type) -> Option<(TokenStream2, u32)> {
    let s = type_to_string(ty);
    match s.as_str() {
        "bool" => Some((quote! { cyclonedds::TYPE_1BY }, 1)),
        "u8" => Some((quote! { cyclonedds::TYPE_1BY }, 1)),
        "i8" => Some((quote! { cyclonedds::TYPE_1BY | cyclonedds::OP_FLAG_SGN }, 1)),
        "u16" => Some((quote! { cyclonedds::TYPE_2BY }, 2)),
        "i16" => Some((quote! { cyclonedds::TYPE_2BY | cyclonedds::OP_FLAG_SGN }, 2)),
        "u32" => Some((quote! { cyclonedds::TYPE_4BY }, 4)),
        "i32" => Some((quote! { cyclonedds::TYPE_4BY | cyclonedds::OP_FLAG_SGN }, 4)),
        "u64" => Some((quote! { cyclonedds::TYPE_8BY }, 8)),
        "i64" => Some((quote! { cyclonedds::TYPE_8BY | cyclonedds::OP_FLAG_SGN }, 8)),
        _ => None,
    }
}

/// Return the primitive typecode for a union case member type, or None if it
/// is a composite (struct implementing DdsType) or String.
fn case_member_typecode(ty: &Type) -> Option<TokenStream2> {
    let s = type_to_string(ty);
    match s.as_str() {
        "i8" => Some(quote! { cyclonedds::TYPE_1BY | cyclonedds::OP_FLAG_SGN }),
        "u8" => Some(quote! { cyclonedds::TYPE_1BY }),
        "i16" => Some(quote! { cyclonedds::TYPE_2BY | cyclonedds::OP_FLAG_SGN }),
        "u16" => Some(quote! { cyclonedds::TYPE_2BY }),
        "i32" => Some(quote! { cyclonedds::TYPE_4BY | cyclonedds::OP_FLAG_SGN }),
        "u32" => Some(quote! { cyclonedds::TYPE_4BY }),
        "i64" => Some(quote! { cyclonedds::TYPE_8BY | cyclonedds::OP_FLAG_SGN }),
        "u64" => Some(quote! { cyclonedds::TYPE_8BY }),
        "f32" => Some(quote! { cyclonedds::TYPE_4BY | cyclonedds::OP_FLAG_FP }),
        "f64" => Some(quote! { cyclonedds::TYPE_8BY | cyclonedds::OP_FLAG_FP }),
        "bool" => Some(quote! { cyclonedds::TYPE_1BY }),
        _ => None,
    }
}

fn derive_union_impl(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let type_name_str = name.to_string();
    let native_name = format_ident!("__CycloneDdsNative{}", name);
    let native_value_name = format_ident!("__CycloneDdsNative{}Value", name);

    let data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "DdsUnion can only be derived for enums",
            ))
        }
    };

    // Parse #[dds_discriminant(Type)]
    let disc_ty: Type = input
        .attrs
        .iter()
        .find(|a| a.path().segments.last().map(|s| s.ident == "dds_discriminant").unwrap_or(false))
        .ok_or_else(|| syn::Error::new_spanned(&input.ident, "DdsUnion requires #[dds_discriminant(Type)] attribute"))?
        .parse_args()?;

    let (disc_typecode, _disc_size) = discriminant_info(&disc_ty)
        .ok_or_else(|| syn::Error::new_spanned(&disc_ty, "unsupported discriminant type; expected bool, u8, i8, u16, i16, u32, i32, u64, or i64"))?;

    // Parse variants
    let mut cases: Vec<(/*value*/ i64, /*variant ident*/ syn::Ident, /*field type*/ Type, /*is_default*/ bool)> = Vec::new();
    let mut default_variant_idx: Option<usize> = None;

    for (i, variant) in data.variants.iter().enumerate() {
        let is_default = variant.attrs.iter().any(|a| {
            a.path().segments.last().map(|s| s.ident == "dds_default").unwrap_or(false)
        });

        let case_value = if is_default {
            // default case does not need a value; we use a sentinel
            -1i64
        } else {
            let case_attr = variant.attrs.iter().find(|a| {
                a.path().segments.last().map(|s| s.ident == "dds_case").unwrap_or(false)
            }).ok_or_else(|| syn::Error::new_spanned(
                variant,
                "DdsUnion variant must have #[dds_case(N)] or #[dds_default]",
            ))?;

            let expr: Expr = case_attr.parse_args()?;
            parse_int_literal(&expr)?
        };

        // Each variant must have exactly one unnamed field (newtype)
        let field_ty = match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                fields.unnamed.first().unwrap().ty.clone()
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    variant,
                    "DdsUnion variant must have exactly one unnamed field, e.g. IntValue(i32)",
                ))
            }
        };

        if is_default {
            if default_variant_idx.is_some() {
                return Err(syn::Error::new_spanned(variant, "DdsUnion can have at most one #[dds_default] variant"));
            }
            default_variant_idx = Some(i);
        }

        cases.push((case_value, variant.ident.clone(), field_ty, is_default));
    }

    if cases.is_empty() {
        return Err(syn::Error::new_spanned(&input.ident, "DdsUnion enum must have at least one variant"));
    }

    let n_cases = cases.len();
    let has_default = default_variant_idx.is_some();

    // Generate native union fields
    let union_arm_names: Vec<syn::Ident> = cases.iter()
        .map(|(_, ident, _, _)| format_ident!("{}_arm", ident.to_string().to_lowercase()))
        .collect();
    let union_arm_types: Vec<&Type> = cases.iter().map(|(_, _, ty, _)| ty).collect();

    // Build ops
    // Layout:
    //   ADR|TYPE_UNI, offset_of_disc, disc_offset(0), (n_cases<<16)|default_jump
    //   JEQ4|case_val_0, jump_to_case_0_ops
    //   JEQ4|case_val_1, jump_to_case_1_ops
    //   ...
    //   [default case ops]   // if has default
    //   [case 0 ops]         // inline ops for case 0 member
    //   [case 1 ops]         // inline ops for case 1 member
    //   ...
    //   RTS
    //
    // The header occupies 4 words.
    // Each JEQ4 occupies 2 words.
    // So the first case ops start at index = 4 + 2*n_cases.
    //
    // We need to compute the jump offsets.
    // For the default case: it comes right after JEQ4 entries.
    // For each numbered case i: it comes after default ops (if any) + all previous case ops.

    // Determine the native representation for each case member
    // Primitives: direct
    // String: uses DdsString (pointer, 8 bytes on 64-bit)
    // Struct (DdsType): uses the struct type directly

    // For each case, compute:
    //   - whether it needs an arena-based write (String/composite)
    //   - the ops words for the case member
    //   - the clone_out logic
    //   - the write_to_native logic

    let header_words: u32 = 4;
    let jeq4_words_per_case: u32 = 2;
    let jeq4_total: u32 = jeq4_words_per_case * (n_cases as u32);
    let after_jeq4 = header_words + jeq4_total;

    // Compute case ops sizes and collect their generated code
    let mut case_ops_parts: Vec<TokenStream2> = Vec::new();

    for (_val, _ident, field_ty, _is_default) in cases.iter() {
        let is_string = is_direct_string(field_ty);

        if is_string {
            // String case: ops = ADR|TYPE_STR, offset
            case_ops_parts.push(quote! {
                __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_STR);
                __ops.push(0u32); // offset patched at runtime
            });
        } else if let Some(typecode) = case_member_typecode(field_ty) {
            // Primitive case: ops = ADR|typecode, offset
            case_ops_parts.push(quote! {
                __ops.push(cyclonedds::OP_ADR | #typecode);
                __ops.push(0u32); // offset patched at runtime
            });
        } else {
            // Composite case: not supported in this first version
            unreachable!();
        }
    }

    // Compute jump targets
    // After the JEQ4 entries, the layout is:
    //   [default case ops]    if has_default
    //   [case 0 ops]
    //   [case 1 ops]
    //   ...
    //   RTS

    // Generate variant idents for clone_out / write_to_native
    let variant_idents: Vec<&syn::Ident> = cases.iter().map(|(_, ident, _, _)| ident).collect();
    let case_values: Vec<i64> = cases.iter().map(|(v, _, _, _)| *v).collect();

    // For the default sentinel, we pick a value that's not any of the explicit case values.
    // We just use the max case value + 1 (or 0 if no cases).
    let default_sentinel = if has_default {
        let max_case = case_values.iter()
            .filter(|v| **v >= 0)
            .max()
            .map(|v| *v as u32)
            .unwrap_or(0);
        let sentinel = max_case + 1;
        quote! { #sentinel }
    } else {
        quote! { 0u32 }
    };

    // For the JEQ4 entries, compute jump offsets
    // The ops after JEQ4 entries:
    //   default_ops (if has_default, 2 or 3 words)
    //   case_0_ops (2 or 3 words)
    //   case_1_ops (2 or 3 words)
    //   ...
    //   RTS
    //
    // case i starts at: after_jeq4 + default_size + sum(previous case sizes)
    // default starts at: after_jeq4

    // Since composite cases have variable-size ops, we compute offsets at runtime.
    // For this first version, we restrict to primitive + String only (fixed 2-word ops per case).
    // This simplifies offset calculation significantly.

    // Verify no composite types for this first version
    let has_composite = cases.iter().any(|(_, _, ty, _)| {
        !is_direct_string(ty) && case_member_typecode(ty).is_none()
    });

    if has_composite {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "DdsUnion first version only supports primitive and String case members; \
             composite (struct) members will be added in a follow-up",
        ));
    }

    // All cases are 2 words each (primitive or string).
    // After JEQ4 entries:
    //   [default ops: 2 words]   if has_default
    //   [case 0 ops: 2 words]
    //   [case 1 ops: 2 words]
    //   ...
    //   RTS

    let case_ops_size = 2u32; // each case is 2 words for primitive/string

    // Default starts at after_jeq4
    // Case i starts at after_jeq4 + (if has_default { case_ops_size } else { 0 }) + i * case_ops_size
    let default_start = after_jeq4;

    // Build the ordered list of ops after JEQ4:
    // First default (if any), then non-default cases in order
    let mut ordered_case_indices: Vec<usize> = Vec::new();

    // Default case ops first
    if let Some(di) = default_variant_idx {
        ordered_case_indices.push(di);
    }
    // Then all non-default cases
    for (i, (_, _, _, is_default)) in cases.iter().enumerate() {
        if !is_default {
            ordered_case_indices.push(i);
        }
    }

    let ordered_case_ops: Vec<TokenStream2> = ordered_case_indices.iter().map(|&idx| {
        case_ops_parts[idx].clone()
    }).collect();

    // Now recompute the JEQ4 entries with correct indices.
    // For non-default case at original index `i`, its position in ordered_case_indices
    // determines its jump target.
    let mut case_jump_targets: Vec<u32> = vec![0; n_cases];
    for (ordered_pos, &orig_idx) in ordered_case_indices.iter().enumerate() {
        let target = default_start + (ordered_pos as u32) * case_ops_size;
        case_jump_targets[orig_idx] = target;
    }

    let jeq4_entries_fixed: Vec<TokenStream2> = cases.iter().enumerate().map(|(i, (_val, _, _, is_default))| {
        if *is_default {
            quote! {}
        } else {
            let val = *_val as u32;
            let target = case_jump_targets[i];
            quote! {
                __ops.push(cyclonedds::OP_JEQ4);
                __ops.push(#val);
                __ops.push(0u32);
                __ops.push(#target);
            }
        }
    }).collect();

    // Generate the clone_out native reading: read discriminator, then read the correct union arm
    let clone_out_arms: Vec<TokenStream2> = cases.iter().enumerate().map(|(i, (_, _, _, is_default))| {
        let variant = &variant_idents[i];
        let arm_name = &union_arm_names[i];
        let is_string = is_direct_string(&cases[i].2);
        if *is_default {
            if is_string {
                quote! { _ => #name::#variant(__native.__value.#arm_name.to_string_lossy()) }
            } else {
                quote! { _ => #name::#variant(::std::ptr::read(&__native.__value.#arm_name)) }
            }
        } else {
            let val = case_values[i] as u32;
            if is_string {
                quote! { #val => #name::#variant(__native.__value.#arm_name.to_string_lossy()) }
            } else {
                quote! { #val => #name::#variant(::std::ptr::read(&__native.__value.#arm_name)) }
            }
        }
    }).collect();

    // Generate write_to_native arms
    let write_arms: Vec<TokenStream2> = cases.iter().enumerate().map(|(i, (_, _, _, is_default))| {
        let variant = &variant_idents[i];
        let arm_name = &union_arm_names[i];
        let is_string = is_direct_string(&cases[i].2);

        if *is_default {
            quote! {
                #name::#variant(ref value) => {
                    __native.__disc = __disc_default_sentinel as #disc_ty;
                    __native.__value.#arm_name = {
                        if #is_string {
                            cyclonedds::DdsString::new(value)?
                        } else {
                            ::std::clone::Clone::clone(value)
                        }
                    };
                }
            }
        } else {
            let val = case_values[i] as u32;
            quote! {
                #name::#variant(ref value) => {
                    __native.__disc = #val as #disc_ty;
                    __native.__value.#arm_name = {
                        if #is_string {
                            cyclonedds::DdsString::new(value)?
                        } else {
                            ::std::clone::Clone::clone(value)
                        }
                    };
                }
            }
        }
    }).collect();

    let n_cases_literal = n_cases as u32;
    let union_header_word3 = if has_default {
        let default_target = default_start;
        quote! { (#n_cases_literal << 16) | #default_target }
    } else {
        quote! { (#n_cases_literal << 16) }
    };

    let expanded = quote! {
        #[repr(C)]
        struct #native_name {
            __disc: #disc_ty,
            __value: #native_value_name,
        }

        #[repr(C)]
        union #native_value_name {
            #(
                #union_arm_names: #union_arm_types,
            )*
        }

        impl Default for #native_value_name {
            fn default() -> Self {
                // Unions can only have one field initialized; zero the first arm.
                unsafe { ::std::mem::zeroed() }
            }
        }

        impl cyclonedds::DdsType for #name {
            fn type_name() -> &'static str { #type_name_str }

            fn ops() -> ::std::vec::Vec<u32> {
                let mut __ops = ::std::vec::Vec::new();
                // ADR | TYPE_UNI header
                __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_UNI | #disc_typecode);
                __ops.push(::std::mem::offset_of!(#native_name, __disc) as u32);
                __ops.push(0u32); // disc offset (relative to start of union), 0 since disc is first
                __ops.push(#union_header_word3);
                // JEQ4 entries
                #(#jeq4_entries_fixed)*
                // Ordered case ops (default first if present, then others)
                #(#ordered_case_ops)*
                // RTS
                __ops.push(cyclonedds::OP_RTS);
                __ops
            }

            fn descriptor_size() -> u32 {
                ::std::mem::size_of::<#native_name>() as u32
            }

            fn descriptor_align() -> u32 {
                ::std::mem::align_of::<#native_name>() as u32
            }

            unsafe fn clone_out(ptr: *const Self) -> Self {
                let __native = &*(ptr as *const #native_name);
                let __disc = __native.__disc as u32;
                match __disc {
                    #(#clone_out_arms)*
                    _ => ::std::panic!("DdsUnion clone_out: unknown discriminator"),
                }
            }

            fn write_to_native<'a>(
                &'a self,
                arena: &'a mut cyclonedds::write_arena::WriteArena,
            ) -> cyclonedds::DdsResult<*const ::std::ffi::c_void> {
                // Compute default sentinel value
                let __disc_default_sentinel: u32 = #default_sentinel;
                let mut __native = #native_name {
                    __disc: 0 as #disc_ty,
                    __value: #native_value_name::default(),
                };
                match self {
                    #(#write_arms)*
                }
                Ok(arena.hold(__native) as *const #native_name as *const ::std::ffi::c_void)
            }
        }
    };

    Ok(expanded)
}

// ---------------------------------------------------------------------------
// DdsBitmask derive
// ---------------------------------------------------------------------------

fn derive_bitmask_impl(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let type_name_str = name.to_string();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(f) => &f.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "DdsBitmask only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "DdsBitmask can only be derived for structs",
            ))
        }
    };

    // Parse #[bit_bound(N)], default 32
    let bit_bound: u32 = input
        .attrs
        .iter()
        .find(|a| a.path().segments.last().map(|s| s.ident == "bit_bound").unwrap_or(false))
        .map(|a| {
            let expr: Expr = a.parse_args()?;
            parse_int_literal(&expr).map(|v| v as u32)
        })
        .transpose()?
        .unwrap_or(32);

    if ![8, 16, 32, 64].contains(&bit_bound) {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "bit_bound must be one of: 8, 16, 32, 64",
        ));
    }

    // Collect field names and validate all are bool
    let field_names: Vec<&syn::Ident> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
    let n_fields = field_names.len();
    if n_fields > (bit_bound as usize) {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("DdsBitmask with bit_bound={} can have at most {} fields, found {}", bit_bound, bit_bound, n_fields),
        ));
    }

    for field in fields {
        let ty_str = type_to_string(&field.ty);
        if ty_str != "bool" {
            return Err(syn::Error::new_spanned(
                field,
                "DdsBitmask fields must all be of type bool",
            ));
        }
    }

    // Determine the backing integer type
    let (backing_ty, _backing_typecode) = match bit_bound {
        8 => (quote! { u8 }, quote! { cyclonedds::TYPE_1BY }),
        16 => (quote! { u16 }, quote! { cyclonedds::TYPE_2BY }),
        32 => (quote! { u32 }, quote! { cyclonedds::TYPE_4BY }),
        64 => (quote! { u64 }, quote! { cyclonedds::TYPE_8BY }),
        _ => unreachable!(),
    };

    let max_value: u64 = if bit_bound == 64 { !0u64 } else { (1u64 << bit_bound) - 1 };
    let max_value_u32 = max_value as u32;

    // Bitmask ops: ADR | TYPE_ENU | (bit_bound << OP_FLAG_SZ_SHIFT), offset, max_value
    // The native representation is just the backing integer.
    // We generate a wrapper native struct with the backing integer.

    let native_name = format_ident!("__CycloneDdsNative{}", name);

    // Generate to_bits / from_bits helpers and individual flag accessors
    let flag_getters: Vec<TokenStream2> = field_names.iter().enumerate().map(|(i, fname)| {
        let mask = 1u64 << i;
        quote! {
            pub fn #fname(&self) -> bool {
                (self.__bits & #mask) != 0
            }
        }
    }).collect();

    let flag_setters: Vec<TokenStream2> = field_names.iter().enumerate().map(|(i, fname)| {
        let method_name = format_ident!("set_{}", fname);
        let mask = 1u64 << i;
        quote! {
            pub fn #method_name(&mut self, value: bool) {
                if value {
                    self.__bits |= #mask;
                } else {
                    self.__bits &= !#mask;
                }
            }
        }
    }).collect();

    // Generate the to_bits field conversions (for DdsType::write_to_native, uses &self)
    let to_bits_self: Vec<TokenStream2> = field_names.iter().enumerate().map(|(i, fname)| {
        let mask = 1u64 << i;
        quote! {
            if self.#fname { bits |= #mask; }
        }
    }).collect();

    // Generate the from_bits field conversions (static conversion from bits)
    let from_bits_fields: Vec<TokenStream2> = field_names.iter().enumerate().map(|(i, fname)| {
        let mask = 1u64 << i;
        quote! {
            #fname: (bits & #mask) != 0,
        }
    }).collect();

    // For the DdsType impl, the bitmask uses TYPE_ENU ops format.
    // The native struct is just the backing integer type.
    // We need a native struct that holds the backing integer.
    let enum_flags_expr = if bit_bound <= 32 {
        quote! { (#bit_bound as u32) << cyclonedds::OP_FLAG_SZ_SHIFT }
    } else {
        // For 64-bit, bit_bound goes in the size shift field too
        quote! { (#bit_bound as u32) << cyclonedds::OP_FLAG_SZ_SHIFT }
    };

    let expanded = quote! {
        #[repr(C)]
        #[derive(Clone)]
        struct #native_name {
            __bits: #backing_ty,
        }

        impl cyclonedds::DdsType for #name {
            fn type_name() -> &'static str { #type_name_str }

            fn ops() -> ::std::vec::Vec<u32> {
                let mut __ops = ::std::vec::Vec::new();
                __ops.push(cyclonedds::OP_ADR | cyclonedds::TYPE_ENU | #enum_flags_expr);
                __ops.push(::std::mem::offset_of!(#native_name, __bits) as u32);
                __ops.push(#max_value_u32);
                __ops.push(cyclonedds::OP_RTS);
                __ops
            }

            fn descriptor_size() -> u32 {
                ::std::mem::size_of::<#native_name>() as u32
            }

            fn descriptor_align() -> u32 {
                ::std::mem::align_of::<#native_name>() as u32
            }

            unsafe fn clone_out(ptr: *const Self) -> Self {
                let __native = &*(ptr as *const #native_name);
                let bits = __native.__bits as u64;
                Self {
                    #(#from_bits_fields)*
                }
            }

            fn write_to_native<'a>(
                &'a self,
                arena: &'a mut cyclonedds::write_arena::WriteArena,
            ) -> cyclonedds::DdsResult<*const ::std::ffi::c_void> {
                let mut bits: u64 = 0;
                #(#to_bits_self)*
                let native = #native_name { __bits: bits as #backing_ty };
                Ok(arena.hold(native) as *const #native_name as *const ::std::ffi::c_void)
            }
        }

        impl #name {
            /// Convert the bitmask to its raw bits representation.
            pub fn to_bits(&self) -> u64 {
                let mut bits: u64 = 0;
                #(#to_bits_self)*
                bits
            }

            /// Convert from raw bits.
            pub fn from_bits(bits: u64) -> Self {
                Self {
                    #(#from_bits_fields)*
                }
            }

            #(#flag_getters)*
            #(#flag_setters)*
        }
    };

    Ok(expanded)
}

fn field_typecode(field: &Field) -> syn::Result<Option<(TokenStream2, u32)>> {
    let ty = &field.ty;

    if let Some(sequence_typecode) = sequence_typecode(ty)? {
        return Ok(Some((sequence_typecode, 2)));
    }

    let type_str = type_to_string(ty);

    match type_str.as_str() {
        "i8" => Ok(Some((
            quote! { cyclonedds::TYPE_1BY | cyclonedds::OP_FLAG_SGN },
            2,
        ))),
        "u8" => Ok(Some((quote! { cyclonedds::TYPE_1BY }, 2))),
        "i16" => Ok(Some((
            quote! { cyclonedds::TYPE_2BY | cyclonedds::OP_FLAG_SGN },
            2,
        ))),
        "u16" => Ok(Some((quote! { cyclonedds::TYPE_2BY }, 2))),
        "i32" => Ok(Some((
            quote! { cyclonedds::TYPE_4BY | cyclonedds::OP_FLAG_SGN },
            2,
        ))),
        "u32" => Ok(Some((quote! { cyclonedds::TYPE_4BY }, 2))),
        "i64" => Ok(Some((
            quote! { cyclonedds::TYPE_8BY | cyclonedds::OP_FLAG_SGN },
            2,
        ))),
        "u64" => Ok(Some((quote! { cyclonedds::TYPE_8BY }, 2))),
        "f32" => Ok(Some((
            quote! { cyclonedds::TYPE_4BY | cyclonedds::OP_FLAG_FP },
            2,
        ))),
        "f64" => Ok(Some((
            quote! { cyclonedds::TYPE_8BY | cyclonedds::OP_FLAG_FP },
            2,
        ))),
        "bool" => Ok(Some((quote! { cyclonedds::TYPE_1BY }, 2))),
        "DdsString" => Ok(Some((quote! { cyclonedds::TYPE_STR }, 2))),
        "cyclonedds::DdsString" => Ok(Some((quote! { cyclonedds::TYPE_STR }, 2))),
        s if s.starts_with("[u8;") || s.starts_with("[ i8 ;") => {
            Ok(Some((quote! { cyclonedds::TYPE_1BY }, 2)))
        }
        _ => Ok(None),
    }
}

fn field_typecode_from_type(ty: &Type) -> syn::Result<Option<(TokenStream2, u32)>> {
    let fake_field: Field = syn::parse_quote! { field: #ty };
    field_typecode(&fake_field)
}

fn option_inner_type(ty: &Type) -> syn::Result<Option<Type>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "Option" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Ok(None);
    };
    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [arg] = collected_args.as_slice() else {
        return Ok(None);
    };
    let GenericArgument::Type(inner_ty) = arg else {
        return Ok(None);
    };
    Ok(Some(inner_ty.clone()))
}

fn sequence_typecode(ty: &Type) -> syn::Result<Option<TokenStream2>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "DdsSequence" {
        return Ok(None);
    }

    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Err(syn::Error::new_spanned(
            ty,
            "DdsSequence requires a single generic type parameter",
        ));
    };

    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [arg] = collected_args.as_slice() else {
        return Err(syn::Error::new_spanned(
            ty,
            "DdsSequence requires exactly one generic type parameter",
        ));
    };

    let GenericArgument::Type(inner_ty) = arg else {
        return Err(syn::Error::new_spanned(
            ty,
            "DdsSequence generic argument must be a type",
        ));
    };

    let inner_type = type_to_string(inner_ty);
    let typecode = match inner_type.as_str() {
        "i8" => quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_1BY | cyclonedds::OP_FLAG_SGN },
        "u8" => quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_1BY },
        "i16" => {
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_2BY | cyclonedds::OP_FLAG_SGN }
        }
        "u16" => quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_2BY },
        "i32" => {
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_4BY | cyclonedds::OP_FLAG_SGN }
        }
        "u32" => quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_4BY },
        "i64" => {
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_8BY | cyclonedds::OP_FLAG_SGN }
        }
        "u64" => quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_8BY },
        "f32" => quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_4BY | cyclonedds::OP_FLAG_FP },
        "f64" => quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_8BY | cyclonedds::OP_FLAG_FP },
        "bool" => quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_1BY },
        "DdsString" | "cyclonedds::DdsString" => {
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_STR }
        }
        _ => return Ok(None),
    };

    Ok(Some(typecode))
}

fn bounded_sequence_typecode(ty: &Type) -> syn::Result<Option<(TokenStream2, TokenStream2)>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "DdsBoundedSequence" {
        return Ok(None);
    }

    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Err(syn::Error::new_spanned(
            ty,
            "DdsBoundedSequence requires type and const bound parameters",
        ));
    };

    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [elem_arg, bound_arg] = collected_args.as_slice() else {
        return Err(syn::Error::new_spanned(
            ty,
            "DdsBoundedSequence requires exactly two generic parameters",
        ));
    };

    let GenericArgument::Type(inner_ty) = elem_arg else {
        return Err(syn::Error::new_spanned(
            ty,
            "DdsBoundedSequence first generic argument must be a type",
        ));
    };
    let GenericArgument::Const(bound_expr) = bound_arg else {
        return Err(syn::Error::new_spanned(
            ty,
            "DdsBoundedSequence second generic argument must be a const bound",
        ));
    };

    let inner_type = type_to_string(inner_ty);
    let typecode = match inner_type.as_str() {
        "i8" => quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_1BY | cyclonedds::OP_FLAG_SGN },
        "u8" => quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_1BY },
        "i16" => {
            quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_2BY | cyclonedds::OP_FLAG_SGN }
        }
        "u16" => quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_2BY },
        "i32" => {
            quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_4BY | cyclonedds::OP_FLAG_SGN }
        }
        "u32" => quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_4BY },
        "i64" => {
            quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_8BY | cyclonedds::OP_FLAG_SGN }
        }
        "u64" => quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_8BY },
        "f32" => quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_4BY | cyclonedds::OP_FLAG_FP },
        "f64" => quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_8BY | cyclonedds::OP_FLAG_FP },
        "bool" => quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_1BY },
        "DdsString" | "cyclonedds::DdsString" => {
            quote! { cyclonedds::TYPE_BSQ | cyclonedds::SUBTYPE_STR }
        }
        _ => return Ok(None),
    };

    let bound_tokens = quote! { (#bound_expr) as u32 };
    Ok(Some((typecode, bound_tokens)))
}

fn composite_sequence_type(ty: &Type) -> syn::Result<Option<Type>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "DdsSequence" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Ok(None);
    };
    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [arg] = collected_args.as_slice() else {
        return Ok(None);
    };
    let GenericArgument::Type(inner_ty) = arg else {
        return Ok(None);
    };
    if field_type_like_primitive_or_wrapped(inner_ty) {
        return Ok(None);
    }
    Ok(Some(inner_ty.clone()))
}

fn composite_array_type(ty: &Type) -> syn::Result<Option<Type>> {
    let Type::Array(arr) = ty else {
        return Ok(None);
    };
    if field_type_like_primitive_or_wrapped(&arr.elem) {
        return Ok(None);
    }
    Ok(Some((*arr.elem).clone()))
}

fn primitive_or_string_array_info(ty: &Type) -> syn::Result<Option<PrimitiveArrayInfo>> {
    if !matches!(ty, Type::Array(_)) {
        return Ok(None);
    }
    fn recurse(ty: &Type, multiplier: TokenStream2) -> syn::Result<Option<PrimitiveArrayInfo>> {
        let Type::Array(arr) = ty else {
            let elem_str = type_to_string(ty);
            let result = match elem_str.as_str() {
                "i8" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_1BY | cyclonedds::OP_FLAG_SGN },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "u8" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_1BY },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "i16" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_2BY | cyclonedds::OP_FLAG_SGN },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "u16" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_2BY },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "i32" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_4BY | cyclonedds::OP_FLAG_SGN },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "u32" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_4BY },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "i64" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_8BY | cyclonedds::OP_FLAG_SGN },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "u64" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_8BY },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "f32" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_4BY | cyclonedds::OP_FLAG_FP },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "f64" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_8BY | cyclonedds::OP_FLAG_FP },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "bool" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_1BY },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                "DdsString" | "cyclonedds::DdsString" => (
                    quote! { cyclonedds::TYPE_ARR | cyclonedds::SUBTYPE_STR },
                    multiplier,
                    Vec::new(),
                    3u32,
                ),
                _ => return Ok(None),
            };
            return Ok(Some(result));
        };

        let len_expr = &arr.len;
        let next_multiplier = quote! { (#multiplier) * ((#len_expr) as u32) };
        recurse(&arr.elem, next_multiplier)
    }

    recurse(ty, quote! { 1u32 })
}

fn composite_bounded_sequence_type(ty: &Type) -> syn::Result<Option<(Type, TokenStream2)>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "DdsBoundedSequence" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Ok(None);
    };
    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [elem_arg, bound_arg] = collected_args.as_slice() else {
        return Ok(None);
    };
    let GenericArgument::Type(inner_ty) = elem_arg else {
        return Ok(None);
    };
    let GenericArgument::Const(bound_expr) = bound_arg else {
        return Ok(None);
    };
    if field_type_like_primitive_or_wrapped(inner_ty) {
        return Ok(None);
    }
    Ok(Some((inner_ty.clone(), quote! { (#bound_expr) as u32 })))
}

fn nested_sequence_info(
    ty: &Type,
    offset_expr: &TokenStream2,
) -> syn::Result<Option<(TokenStream2, TokenStream2)>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };

    let (outer_type, outer_subtype, inner_ty, outer_bound) = if last.ident == "DdsSequence" {
        let PathArguments::AngleBracketed(args) = &last.arguments else {
            return Ok(None);
        };
        let collected_args = args.args.iter().collect::<Vec<_>>();
        let [arg] = collected_args.as_slice() else {
            return Ok(None);
        };
        let GenericArgument::Type(inner_ty) = arg else {
            return Ok(None);
        };
        (
            quote! { cyclonedds::TYPE_SEQ },
            quote! { cyclonedds::SUBTYPE_SEQ },
            inner_ty.clone(),
            None,
        )
    } else if last.ident == "DdsBoundedSequence" {
        let PathArguments::AngleBracketed(args) = &last.arguments else {
            return Ok(None);
        };
        let collected_args = args.args.iter().collect::<Vec<_>>();
        let [elem_arg, bound_arg] = collected_args.as_slice() else {
            return Ok(None);
        };
        let GenericArgument::Type(inner_ty) = elem_arg else {
            return Ok(None);
        };
        let GenericArgument::Const(bound_expr) = bound_arg else {
            return Ok(None);
        };
        (
            quote! { cyclonedds::TYPE_BSQ },
            quote! { cyclonedds::SUBTYPE_BSQ },
            inner_ty.clone(),
            Some(quote! { (#bound_expr) as u32 }),
        )
    } else {
        return Ok(None);
    };

    if let Some(inner_typecode) = sequence_typecode(&inner_ty)? {
        let (block, len) = if let Some(ref bound_expr) = outer_bound {
            (
                quote! {
                    __ops.push(cyclonedds::OP_ADR | #outer_type | #outer_subtype);
                    __ops.push(#offset_expr);
                    __ops.push(#bound_expr);
                    __ops.push(::std::mem::size_of::<#inner_ty>() as u32);
                    __ops.push((8u32 << 16) + 5u32);
                    __ops.push(cyclonedds::OP_ADR | #inner_typecode);
                    __ops.push(0u32);
                    __ops.push(cyclonedds::OP_RTS);
                },
                quote! { 8u32 },
            )
        } else {
            (
                quote! {
                    __ops.push(cyclonedds::OP_ADR | #outer_type | #outer_subtype);
                    __ops.push(#offset_expr);
                    __ops.push(::std::mem::size_of::<#inner_ty>() as u32);
                    __ops.push((7u32 << 16) + 4u32);
                    __ops.push(cyclonedds::OP_ADR | #inner_typecode);
                    __ops.push(0u32);
                    __ops.push(cyclonedds::OP_RTS);
                },
                quote! { 7u32 },
            )
        };
        return Ok(Some((block, len)));
    }

    if let Some((inner_typecode, inner_bound)) = bounded_sequence_typecode(&inner_ty)? {
        let (block, len) = if let Some(ref bound_expr) = outer_bound {
            (
                quote! {
                    __ops.push(cyclonedds::OP_ADR | #outer_type | #outer_subtype);
                    __ops.push(#offset_expr);
                    __ops.push(#bound_expr);
                    __ops.push(::std::mem::size_of::<#inner_ty>() as u32);
                    __ops.push((9u32 << 16) + 5u32);
                    __ops.push(cyclonedds::OP_ADR | #inner_typecode);
                    __ops.push(0u32);
                    __ops.push(#inner_bound);
                    __ops.push(cyclonedds::OP_RTS);
                },
                quote! { 9u32 },
            )
        } else {
            (
                quote! {
                    __ops.push(cyclonedds::OP_ADR | #outer_type | #outer_subtype);
                    __ops.push(#offset_expr);
                    __ops.push(::std::mem::size_of::<#inner_ty>() as u32);
                    __ops.push((8u32 << 16) + 4u32);
                    __ops.push(cyclonedds::OP_ADR | #inner_typecode);
                    __ops.push(0u32);
                    __ops.push(#inner_bound);
                    __ops.push(cyclonedds::OP_RTS);
                },
                quote! { 8u32 },
            )
        };
        return Ok(Some((block, len)));
    }

    Ok(None)
}

fn enum_sequence_info(ty: &Type) -> syn::Result<Option<Type>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "DdsSequence" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Ok(None);
    };
    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [arg] = collected_args.as_slice() else {
        return Ok(None);
    };
    let GenericArgument::Type(inner_ty) = arg else {
        return Ok(None);
    };
    if field_type_like_primitive_or_wrapped(inner_ty) {
        return Ok(None);
    }
    Ok(Some(inner_ty.clone()))
}

fn enum_bounded_sequence_info(ty: &Type) -> syn::Result<Option<(Type, TokenStream2)>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "DdsBoundedSequence" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Ok(None);
    };
    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [elem_arg, bound_arg] = collected_args.as_slice() else {
        return Ok(None);
    };
    let GenericArgument::Type(inner_ty) = elem_arg else {
        return Ok(None);
    };
    let GenericArgument::Const(bound_expr) = bound_arg else {
        return Ok(None);
    };
    if field_type_like_primitive_or_wrapped(inner_ty) {
        return Ok(None);
    }
    Ok(Some((inner_ty.clone(), quote! { (#bound_expr) as u32 })))
}

fn enum_array_info(ty: &Type) -> syn::Result<Option<(TokenStream2, TokenStream2, TokenStream2)>> {
    fn recurse(ty: &Type, multiplier: TokenStream2) -> syn::Result<Option<(Type, TokenStream2)>> {
        let Type::Array(arr) = ty else {
            if field_type_like_primitive_or_wrapped(ty) {
                return Ok(None);
            }
            return Ok(Some((ty.clone(), multiplier)));
        };
        let len_expr = &arr.len;
        recurse(&arr.elem, quote! { (#multiplier) * ((#len_expr) as u32) })
    }

    let Some((elem_ty, len_expr)) = recurse(ty, quote! { 1u32 })? else {
        return Ok(None);
    };
    Ok(Some((
        len_expr,
        quote! { <#elem_ty as cyclonedds::DdsEnumType>::enum_op_flags() },
        quote! { <#elem_ty as cyclonedds::DdsEnumType>::max_discriminant() },
    )))
}

fn is_direct_string(ty: &Type) -> bool {
    matches!(
        type_to_string(ty).as_str(),
        "String" | "std::string::String"
    )
}

fn direct_vec_info(ty: &Type) -> syn::Result<Option<DirectVecInfo>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "Vec" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Ok(None);
    };
    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [arg] = collected_args.as_slice() else {
        return Ok(None);
    };
    let GenericArgument::Type(inner_ty) = arg else {
        return Ok(None);
    };
    let inner_str = type_to_string(inner_ty);
    let info = match inner_str.as_str() {
        "i8" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_1BY | cyclonedds::OP_FLAG_SGN },
            2u32,
            quote! { cyclonedds::DdsSequence<i8> },
        ),
        "u8" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_1BY },
            2u32,
            quote! { cyclonedds::DdsSequence<u8> },
        ),
        "i16" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_2BY | cyclonedds::OP_FLAG_SGN },
            2u32,
            quote! { cyclonedds::DdsSequence<i16> },
        ),
        "u16" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_2BY },
            2u32,
            quote! { cyclonedds::DdsSequence<u16> },
        ),
        "i32" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_4BY | cyclonedds::OP_FLAG_SGN },
            2u32,
            quote! { cyclonedds::DdsSequence<i32> },
        ),
        "u32" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_4BY },
            2u32,
            quote! { cyclonedds::DdsSequence<u32> },
        ),
        "i64" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_8BY | cyclonedds::OP_FLAG_SGN },
            2u32,
            quote! { cyclonedds::DdsSequence<i64> },
        ),
        "u64" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_8BY },
            2u32,
            quote! { cyclonedds::DdsSequence<u64> },
        ),
        "f32" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_4BY | cyclonedds::OP_FLAG_FP },
            2u32,
            quote! { cyclonedds::DdsSequence<f32> },
        ),
        "f64" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_8BY | cyclonedds::OP_FLAG_FP },
            2u32,
            quote! { cyclonedds::DdsSequence<f64> },
        ),
        "bool" => (
            quote! { cyclonedds::TYPE_SEQ | cyclonedds::SUBTYPE_1BY },
            2u32,
            quote! { cyclonedds::DdsSequence<bool> },
        ),
        _ => return Ok(None),
    };
    Ok(Some(info))
}

fn is_direct_vec_string(ty: &Type) -> syn::Result<Option<()>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "Vec" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Ok(None);
    };
    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [arg] = collected_args.as_slice() else {
        return Ok(None);
    };
    let GenericArgument::Type(inner_ty) = arg else {
        return Ok(None);
    };
    if is_direct_string(inner_ty) {
        Ok(Some(()))
    } else {
        Ok(None)
    }
}

fn enum_direct_vec_info(ty: &Type) -> syn::Result<Option<Type>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "Vec" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Ok(None);
    };
    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [arg] = collected_args.as_slice() else {
        return Ok(None);
    };
    let GenericArgument::Type(inner_ty) = arg else {
        return Ok(None);
    };
    if field_type_like_primitive_or_wrapped(inner_ty) {
        return Ok(None);
    }
    Ok(Some(inner_ty.clone()))
}

fn direct_vec_composite_info(ty: &Type) -> syn::Result<Option<Type>> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return Ok(None);
    };
    let Some(last) = path.segments.last() else {
        return Ok(None);
    };
    if last.ident != "Vec" {
        return Ok(None);
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return Ok(None);
    };
    let collected_args = args.args.iter().collect::<Vec<_>>();
    let [arg] = collected_args.as_slice() else {
        return Ok(None);
    };
    let GenericArgument::Type(inner_ty) = arg else {
        return Ok(None);
    };
    if field_type_like_primitive_or_wrapped(inner_ty) || is_direct_string(inner_ty) {
        return Ok(None);
    }
    Ok(Some(inner_ty.clone()))
}

fn field_type_like_primitive_or_wrapped(ty: &Type) -> bool {
    if sequence_typecode(ty).ok().flatten().is_some()
        || bounded_sequence_typecode(ty).ok().flatten().is_some()
    {
        return true;
    }
    let type_str = type_to_string(ty);
    matches!(
        type_str.as_str(),
        "i8" | "u8"
            | "i16"
            | "u16"
            | "i32"
            | "u32"
            | "i64"
            | "u64"
            | "f32"
            | "f64"
            | "bool"
            | "DdsString"
            | "cyclonedds::DdsString"
    ) || type_str.starts_with("[u8;")
        || type_str.starts_with("[ i8 ;")
}

fn parse_int_literal(expr: &Expr) -> syn::Result<i64> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(lit), ..
        }) => lit.base10_parse::<i64>(),
        _ => Err(syn::Error::new_spanned(
            expr,
            "DdsEnum discriminants must be integer literals in the initial implementation",
        )),
    }
}

fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Path(TypePath { path, .. }) => {
            let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            segments.join("::")
        }
        Type::Array(arr) => {
            let elem = type_to_string(&arr.elem);
            let len = match &arr.len {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(lit),
                    ..
                }) => lit.base10_digits().to_string(),
                _ => "?".to_string(),
            };
            format!("[{}; {}]", elem, len)
        }
        _ => format!("<unknown type: {}>", quote::quote!(#ty)),
    }
}
