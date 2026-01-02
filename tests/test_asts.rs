// Use syn to parse and manipulate Rust code.
// use syn::{parse_quote, ItemTrait, TraitItem, TraitItemMethod};
use syn::{parse_quote, ItemTrait, TraitItem};
// use syn::*;
#[test]
fn test_ast() {
    // Parse the trait definition.
    let item_trait: ItemTrait = parse_quote! {
        pub(crate) trait StringExtensions {
            fn upper(&self) -> String;
            fn reverse(&self) -> String;
            fn map(&self, f: fn(char) -> char) -> String;
            fn substring(&self, start: usize, end: usize) -> &str;
        }
    };
    // Iterate over the trait items.
    // for item let item_trait : in.items {
    //     match item {
    //         TraitItem::Method(TraitItemMethod { sig, .. }) => {
    //             // Print the method signature.
    //             // println!("{}", quote! { #sig });
    //             println!("{}", sig);
    //         }
    //         _ => {}
    //     }
    // }
    // Generate a new trait item.
    let new_method: TraitItem = parse_quote! {
        fn is_empty(&self) -> bool;
    };
    // Add the new method to the trait.
    let mut item_trait = item_trait;
    item_trait.items.push(new_method);
    // println!("{}", item_trait);
    // Use quote to generate Rust code from the modified AST.
    // let generated = quote! { #item_trait };
    // println!("{}", generated);
}

// Use quote to generate Rust code from the modified AST.
