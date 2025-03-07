use prettyplease::unparse;

#[allow(unused)]
/// Uses prettyplease to format the code.
/// If prettyplease can't format the code, it will panic in all cases.
/// If that happens, use default_fmt instead which is less pretty but more reliable.
pub fn fmt(code: proc_macro2::TokenStream) -> String {
    let file = syn::parse_file(&code.to_string().replace(" } ", "\n}")).unwrap();
    unparse(&file)
}

#[allow(unused)]
/// A simple formatter that just inserts newlines and indentation in most cases.
/// Does not work on unbalanced scopes inside of strings and other edge cases.
pub fn default_fmt(s: &proc_macro2::TokenStream) -> String {
    let s = s
        .to_string()
        .replace(" <", "<")
        .replace(" >", ">")
        .replace("( ", "(\n")
        .replace(" ) ", "\n)")
        .replace("{ ", "{\n")
        .replace("} ", "\n}\n")
        .replace("; ", ";\n")
        .replace(" , ", ",\n");
    let split = s.split('\n');
    let mut result = String::new();
    let mut curly = 0;
    let mut parens = 0;
    let mut angled = 0;
    for line in split {
        if line.is_empty() {
            continue;
        }
        curly += line.match_indices('{').count() as i32 - line.match_indices('}').count() as i32;
        parens += line.match_indices('(').count() as i32 - line.match_indices(')').count() as i32;
        let angled_offset = line.match_indices('<').count() as i32
            - line.match_indices('>').count() as i32
            + line.match_indices("->").count() as i32;
        result.push_str(line);
        angled += angled_offset;
        if angled == 0 {
            result.push('\n');
            for _ in 0..curly + parens {
                result.push_str("    ");
            }
        }
    }
    result
}
