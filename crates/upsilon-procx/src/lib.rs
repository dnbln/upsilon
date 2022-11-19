use proc_macro2::TokenStream;
use std::time::SystemTime;

#[proc_macro]
pub fn private_context(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime went before UNIX_EPOCH");

    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let pid = std::process::id();

    let name = quote::format_ident!("__private_context_{}_{}_{}", secs, nanos, pid);
    let ts = TokenStream::from(item);

    quote::quote!(
        mod #name {
            #ts
        }
    )
    .into()
}
