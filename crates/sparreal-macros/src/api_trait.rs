#[cfg(test)]
mod tests {

    use syn::TraitItem;

    use super::*;

    #[test]
    fn it_works() {
        let f = syn::parse_str::<syn::ItemTrait>(
            "pub trait Platform2 {
    fn current_ticks(a: usize) -> u64;
}",
        )
        .unwrap();

        println!("{:?}", f);

        for item in &f.items {
            if let TraitItem::Fn(func) = item {
                println!("{}", func.sig.ident);

                println!("{:?}", func.sig.inputs.first());
            }
        }
    }

    #[test]
    fn it_works2() {
        let f = syn::parse_str::<syn::ItemImpl>(
            "impl Platform2 for Platform2Impl {
    fn current_ticks(a: usize) -> u64 {
        0
    }
}",
        )
        .unwrap();

        println!("{:?}", f);

       
    }
}
