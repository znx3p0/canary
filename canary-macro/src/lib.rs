use proc_macro::{Span, TokenStream};
use proc_macro_error::proc_macro_error;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, ItemFn, LitStr, Type};

macro_rules! panic_span {
    ($span: expr, $msg: expr) => {
        return syn::Error::new($span.span(), $msg)
            .to_compile_error()
            .into()
    };
}

/// services are methods that run on a global cluster and can be exposed through providers.
///
/// At first sight, they might look similar to HTTP handlers, and although they are similar,
/// it is important to note that there are various differences.
///
/// A service represents a pipeline of objects through which objects may be sent or received.
///
/// HTTP handlers are stuck in TCP, services can use TCP, Unix and other available providers.
/// HTTP handlers are based on the request-response architecture, services are object-stream based.
///
/// ```norun
/// #[canary::service]
/// async fn ping(mut chan: Channel) -> Result<()> {
///     chan.send(123).await?;
///     Ok(())
/// }
/// ```
///
/// Services can also be coupled with metadata
/// ```norun
/// #[canary::service]
/// async fn send_magic_number(mut chan: Channel, magic_num: u32) -> Result<()> {
///     chan.send(magic_num).await?;
///     Ok(())
/// }
/// ```
///
/// And they can also have a context to the route in which they are stored
/// ```norun
/// #[canary::service]
/// async fn auth_service(mut chan: Channel, _: (), ctx: Ctx) -> Result<()> {
///     let magic_num: u32 = chan.receive().await?;
///     if magic_num == 10 {
///         // send the channel to the ping service on the current route
///         ctx.switch("ping", chan.bare()).ok();
///     }
///     Ok(())
/// }
/// ```
#[proc_macro_error]
#[proc_macro_attribute]
pub fn service(attrs: TokenStream, tokens: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(tokens as ItemFn);
    let vis = &item.vis;

    // let mut has_pipeline = true;
    let pipeline = match syn::parse::<Type>(attrs) {
        Ok(t) => t,
        Err(_) => {
            // has_pipeline = false;
            Type::Verbatim(quote!(()))
        }
    };

    let endpoint = LitStr::new(
        &format!("{}", item.sig.ident.clone()),
        Span::call_site().into(),
    );

    let name = item.sig.ident.clone();
    if let None = item.sig.asyncness {
        panic_span!(item.sig, "function has to be async");
    }

    let (
        (chan_mut, chan_ident, chan_ty),
        (ctx_mut, ctx_ident, ctx_ty),
        (meta_mut, meta_ident, meta_ty),
    ) = {
        let (mut chan_mut, mut chan_ident, mut chan_ty) = (
            None,
            format_ident!("__canary_inner_channel"),
            quote!(::canary::Channel),
        );
        let (mut meta_mut, mut meta_ident, mut meta_ty) =
            (None, format_ident!("__canary_inner_meta"), quote!(()));
        let (mut ctx_mut, mut ctx_ident, mut ctx_ty) = (
            None,
            format_ident!("__canary_inner_context"),
            quote!(::canary::Ctx),
        );
        let mut counter = 0;
        for item in item.sig.inputs {
            match item {
                syn::FnArg::Typed(pat) => {
                    let tty = pat.ty;
                    let (mutability, ident, ty) = match *pat.pat {
                        syn::Pat::Ident(ident) => {
                            let mutability = ident.mutability;
                            let ident = ident.ident;
                            (mutability, ident, quote!(#tty))
                        }
                        syn::Pat::Wild(_) => {
                            let mutability = None;
                            let ident = format_ident!("_");
                            (mutability, ident, quote!(#tty))
                        }
                        syn::Pat::Struct(ident) => {
                            proc_macro_error::abort!(
                                ident.span(),
                                "destructuring in services is not supported yet"
                            );
                        }
                        _ => panic!("error parsing inputs"),
                    };
                    match counter {
                        0 => {
                            chan_mut = mutability;
                            chan_ident = ident;
                            chan_ty = ty;
                        }
                        1 => {
                            meta_ident = ident;
                            meta_ty = ty;
                            meta_mut = mutability;
                        }
                        2 => {
                            ctx_ident = ident;
                            ctx_ty = ty;
                            ctx_mut = mutability;
                        }
                        _ => break,
                    }
                }
                syn::FnArg::Receiver(_) => panic!("invalid type"),
            }
            counter += 1;
        }
        (
            (chan_mut, chan_ident, chan_ty),
            (ctx_mut, ctx_ident, ctx_ty),
            (meta_mut, meta_ident, meta_ty),
        )
    };

    let block = item.block;
    let ret = {
        match item.sig.output {
            syn::ReturnType::Default => quote!(()),
            syn::ReturnType::Type(_, ty) => quote!(#ty),
        }
    };

    quote!(
        #[allow(non_camel_case_types)]
        #vis struct #name;
        #[cfg(not(target_arch = "wasm32"))]
        impl ::canary::service::Service for #name {
            const ENDPOINT: &'static str = #endpoint;
            type Pipeline = #pipeline;
            type Meta = #meta_ty;
            fn service(__canary_meta: Self::Meta) -> ::canary::service::Svc {
                fn __inner_canary_check_res<A, B, C, F: std::future::Future<Output = #ret>>(f: impl Fn(A, B, C) -> F + Clone) -> impl Fn(A, B, C) -> F + Clone { f }
                let func = __inner_canary_check_res(
                    |#meta_mut #meta_ident: #meta_ty, #chan_mut #chan_ident: #chan_ty, #ctx_mut #ctx_ident: #ctx_ty| async move {
                        #block
                    }
                );
                ::canary::service::run_metadata(
                    __canary_meta,
                    func
                )
            }
        }
    )
    .into()
}

#[proc_macro_attribute]
pub fn main(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let inputs = &input.sig.inputs;

    if input.sig.asyncness.is_none() {
        let msg = "the async keyword is missing from the function declaration";
        return syn::Error::new_spanned(input.sig.fn_token, msg)
            .to_compile_error()
            .into();
    } else if name == "main" && !inputs.is_empty() {
        let msg = "the main function cannot accept arguments";
        return syn::Error::new_spanned(&input.sig.inputs, msg)
            .to_compile_error()
            .into();
    }

    quote!(
        fn main() #ret {
            #input
            ::canary::runtime::block_on(main())
        }
    )
    .into()
}
