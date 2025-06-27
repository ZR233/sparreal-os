extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate core;
extern crate proc_macro2;
#[macro_use]
extern crate syn;

mod api_trait;
mod arch;

use darling::{FromMeta, ast::NestedMeta};
use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{FnArg, ItemFn, PathArguments, Type, Visibility, parse, spanned::Spanned};

/// Attribute to declare the entry point of the program
///
/// **IMPORTANT**: This attribute must appear exactly *once* in the dependency graph. Also, if you
/// are using Rust 1.30 the attribute must be used on a reachable item (i.e. there must be no
/// private modules between the item and the root of the crate); if the item is in the root of the
/// crate you'll be fine. This reachability restriction doesn't apply to Rust 1.31 and newer releases.
///
/// The specified function will be called by the reset handler *after* RAM has been initialized.
/// If present, the FPU will also be enabled before the function is called.
///
/// The type of the specified function must be `[unsafe] fn() -> !` (never ending function)
///
/// # Properties
///
/// The entry point will be called by the reset handler. The program can't reference to the entry
/// point, much less invoke it.
///
/// # Examples
///
/// - Simple entry point
///
/// ``` no_run
/// # #![no_main]
/// # use sparreal_macros::entry;
/// #[entry]
/// fn main() -> ! {
///     loop {
///         /* .. */
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as ItemFn);

    // check the function arguments
    if f.sig.inputs.len() > 3 {
        return parse::Error::new(
            f.sig.inputs.last().unwrap().span(),
            "`#[entry]` function has too many arguments",
        )
        .to_compile_error()
        .into();
    }
    for arg in &f.sig.inputs {
        match arg {
            FnArg::Receiver(_) => {
                return parse::Error::new(arg.span(), "invalid argument")
                    .to_compile_error()
                    .into();
            }
            FnArg::Typed(t) => {
                if !is_simple_type(&t.ty, "usize") {
                    return parse::Error::new(t.ty.span(), "argument type must be usize")
                        .to_compile_error()
                        .into();
                }
            }
        }
    }

    // check the function signature
    let valid_signature = f.sig.constness.is_none()
        && f.sig.asyncness.is_none()
        && f.vis == Visibility::Inherited
        && f.sig.abi.is_none()
        && f.sig.generics.params.is_empty()
        && f.sig.generics.where_clause.is_none()
        && f.sig.variadic.is_none()
        // && match f.sig.output {
        //     ReturnType::Default => false,
        //     ReturnType::Type(_, ref ty) => matches!(**ty, Type::Never(_)),
        // }
        ;

    if !valid_signature {
        return parse::Error::new(
            f.span(),
            "`#[entry]` function must have signature `[unsafe] fn([arg0: usize, ...]) `",
        )
        .to_compile_error()
        .into();
    }

    if !args.is_empty() {
        return parse::Error::new(Span::call_site(), "This attribute accepts no arguments")
            .to_compile_error()
            .into();
    }

    // XXX should we blacklist other attributes?
    let attrs = f.attrs;
    let unsafety = f.sig.unsafety;
    let args = f.sig.inputs;
    let stmts = f.block.stmts;

    quote!(
        #[allow(non_snake_case)]
        #[unsafe(no_mangle)]
        #(#attrs)*
        pub #unsafety extern "C" fn __sparreal_rt_main(#args) {
            #(#stmts)*
        }
    )
    .into()
}

#[allow(unused)]
fn is_simple_type(ty: &Type, name: &str) -> bool {
    if let Type::Path(p) = ty
        && p.qself.is_none()
        && p.path.leading_colon.is_none()
        && p.path.segments.len() == 1
    {
        let segment = p.path.segments.first().unwrap();
        if segment.ident == name && segment.arguments == PathArguments::None {
            return true;
        }
    }
    false
}

const NAMESPACE: &str = "sparreal_os";

#[proc_macro_attribute]
pub fn api_trait(_args: TokenStream, item: TokenStream) -> TokenStream {
    abi_singleton::api_trait(item, NAMESPACE)
}

#[proc_macro_attribute]
pub fn api_impl(_args: TokenStream, item: TokenStream) -> TokenStream {
    abi_singleton::api_impl(item, NAMESPACE)
}

#[proc_macro]
pub fn build_test_setup(_input: TokenStream) -> TokenStream {
    quote! {
    println!("cargo::rustc-link-arg-tests=-Tlink.x");
    println!("cargo::rustc-link-arg-tests=-no-pie");
    println!("cargo::rustc-link-arg-tests=-znostart-stop-gc");
    }
    .into()
}

#[proc_macro]
pub fn define_aarch64_tcb_switch(_input: TokenStream) -> TokenStream {
    let fp = arch::aarch64::tcb_switch(true);
    let sp = arch::aarch64::tcb_switch(false);

    quote! {
        #[cfg(hard_float)]
        #fp

        #[cfg(not(hard_float))]
        #sp
    }
    .into()
}

/// A speaking volume. Deriving `FromMeta` will cause this to be usable
/// as a string value for a meta-item key.
#[derive(Debug, Clone, Copy, FromMeta)]
#[darling(default)]
enum Aarch64TrapHandlerKind {
    Irq,
    Fiq,
    Sync,
    #[darling(rename = "serror")]
    SError,
}

#[derive(Debug, FromMeta)]
struct Aarch64TrapHandlerArgs {
    kind: Aarch64TrapHandlerKind,
}

#[proc_macro_attribute]
pub fn aarch64_trap_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let attr_args = match NestedMeta::parse_meta_list(args.into()) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(darling::Error::from(e).write_errors());
        }
    };
    let args = match Aarch64TrapHandlerArgs::from_list(&attr_args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let func = parse_macro_input!(input as ItemFn);

    match args.kind {
        Aarch64TrapHandlerKind::Irq | Aarch64TrapHandlerKind::Fiq => {
            arch::aarch64::trap_handle_irq(func).into()
        }
        Aarch64TrapHandlerKind::Sync => arch::aarch64::trap_handle_irq(func).into(),
        Aarch64TrapHandlerKind::SError => arch::aarch64::trap_handle_irq(func).into(),
    }
}
