//! Comparação de texto: sem acento, sem maiúscula, por começo de palavra.

use unicode_normalization::UnicodeNormalization;

/// "Configurações" -> "configuracoes"
pub fn norm(s: &str) -> String {
    s.nfkd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).flat_map(char::to_lowercase).collect()
}

/// Quebra em palavras: espaço, _ - . / e CamelCase separam ("PokeCroc_Server" -> poke croc server).
pub fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(norm(&cur));
                cur.clear();
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower && !cur.is_empty() {
            out.push(norm(&cur));
            cur.clear();
        }
        prev_lower = c.is_lowercase() || c.is_numeric();
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(norm(&cur));
    }
    out
}

/// Nota de 0 a 1 de quanto `query` (já normalizada) bate com o nome; `extra` (descrição,
/// palavras-chave) vale menos. 0 = não aparece.
pub fn score(query: &str, name: &str, extra: &str) -> f32 {
    if query.is_empty() {
        return 0.5;
    }
    let n = norm(name);
    if n == query {
        return 1.0;
    }
    if n.starts_with(query) {
        return 0.9;
    }
    let qs: Vec<&str> = query.split_whitespace().collect();
    let nw = words(name);
    // a palavra inteira junta também conta: "pokecroc" bate com "PokeCroc_Server"
    let junto = n.replace(|c: char| !c.is_alphanumeric(), "");
    if qs.iter().all(|q| nw.iter().any(|w| w.starts_with(q)) || junto.starts_with(q)) {
        return 0.8;
    }
    // Com 1 ou 2 letras, só começo de palavra: "po" não acha "lstopo" nem a descrição
    if query.chars().count() < 3 {
        return 0.0;
    }
    if n.contains(query) {
        return 0.65;
    }
    let ew = words(extra);
    if qs.iter().all(|q| nw.iter().chain(ew.iter()).any(|w| w.starts_with(q))) {
        return 0.4;
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notas() {
        assert_eq!(norm("Configurações"), "configuracoes");
        assert_eq!(words("PokeCroc_Server"), ["poke", "croc", "server"]);
        assert_eq!(score("kate", "Kate", ""), 1.0);
        assert_eq!(score("kr", "Krita", ""), 0.9);
        assert_eq!(score("server", "PokeCroc_Server", ""), 0.8);
        assert_eq!(score("navegador", "Vivaldi", "Navegador web"), 0.4);
        assert_eq!(score("xyz", "Vivaldi", "Navegador"), 0.0);
        assert_eq!(score("po", "Hardware Locality lstopo", "topologia"), 0.0);
        assert_eq!(score("topo", "Hardware Locality lstopo", ""), 0.65);
    }
}
