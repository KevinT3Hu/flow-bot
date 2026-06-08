use proc_macro::TokenStream;
use syn::{FnArg, ImplItem, ItemImpl, parse_macro_input};

pub fn flow_service_macro(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemImpl);

    let trt = match &item.trait_ {
        Some((_, p, _)) => p,
        None => {
            return syn::Error::new_spanned(
                &item,
                "The #[flow_service] attribute can only be applied to impl blocks for traits",
            )
            .to_compile_error()
            .into();
        }
    };

    let struct_name = match &*item.self_ty {
        syn::Type::Path(type_path) => &type_path.path.segments.last().unwrap().ident,
        _ => {
            return syn::Error::new_spanned(
                &item.self_ty,
                "The #[flow_service] attribute can only be applied to impl blocks for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    // Collect the init function if it exists
    let init_fn = item.items.iter().find_map(|it| {
        let ImplItem::Fn(fn_item) = it else {
            return None;
        };
        if fn_item.sig.ident == "init" {
            Some(fn_item)
        } else {
            None
        }
    });

    let methods = item.items.iter().filter_map(|it| {
        let ImplItem::Fn(fn_item) = it else { return None };

        // Skip the "init" function as it's part of the Service trait
        if fn_item.sig.ident == "init" {
            return None;
        }

        let param_decls = fn_item.sig.inputs.iter().filter_map(|arg| {
            let FnArg::Typed(pat_type) = arg else { return None };
            let syn::Pat::Ident(ident) = &*pat_type.pat else { return None };
            let syn::Type::Path(type_path) = &*pat_type.ty else { return None };

            let param_ident = &ident.ident;
            let ty_ident = &type_path.path.segments.last()?.ident;

            Some(quote::quote! {
                let #param_ident: #ty_ident = ::flow_bot::base::extract::FromEvent::from_event(context.clone(), event.clone()).await.ok_or(::flow_bot::base::handler::HandlerError::skip())?;
            })
        });

        let func_body = &fn_item.block;

        Some(quote::quote! {
            let controller_result = ({
                #(#param_decls)*
                #func_body
            }).into_result();

            if matches!(controller_result, Ok(::flow_bot::base::handler::HandlerControl::Block)) {
                return Ok(::flow_bot::base::handler::HandlerControl::Block);
            }
        })
    });

    quote::quote! {
        #[::async_trait::async_trait]
        impl #trt for #struct_name {
            #init_fn

            async fn serve(&self, context: ::flow_bot::base::context::BotContext, event: ::flow_bot::event::BotEvent) -> Result<::flow_bot::base::handler::HandlerControl, ::flow_bot::base::handler::HandlerError> {
                #(#methods)*
                Ok(::flow_bot::base::handler::HandlerControl::Continue)
            }
        }
    }
    .into()
}
