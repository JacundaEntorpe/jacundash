//! "10 usd em brl", "5 km em milhas", "100 f em c", "2 gb em mb". Enter copia o resultado.
//! A cotação das moedas vem da open.er-api.com (grátis, sem cadastro) e fica guardada por 12 horas.

use crate::config::pasta_estado;
use crate::engine::{Provider, Query};
use crate::matcher;
use crate::model::{Effect, Item};
use crate::providers::calc::formatar;
use crate::sistema;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Conversor {
    cambio: Mutex<Option<Cambio>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Cambio {
    /// Quantas unidades de cada moeda valem 1 dólar.
    taxas: HashMap<String, f64>,
    /// Quando foi buscada (segundos desde 1970).
    quando: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum Tipo {
    Comprimento,
    Massa,
    Dados,
    Tempo,
    Volume,
    Velocidade,
    Area,
    Temperatura,
}

/// (nomes, tipo, quanto vale na unidade base do tipo)
const UNIDADES: &[(&[&str], Tipo, f64)] = &[
    (&["mm", "milimetro", "milimetros"], Tipo::Comprimento, 0.001),
    (&["cm", "centimetro", "centimetros"], Tipo::Comprimento, 0.01),
    (&["m", "metro", "metros"], Tipo::Comprimento, 1.0),
    (&["km", "quilometro", "quilometros"], Tipo::Comprimento, 1000.0),
    (&["in", "pol", "polegada", "polegadas", "inch", "inches"], Tipo::Comprimento, 0.0254),
    (&["ft", "pe", "pes", "feet", "foot"], Tipo::Comprimento, 0.3048),
    (&["yd", "jarda", "jardas"], Tipo::Comprimento, 0.9144),
    (&["mi", "milha", "milhas", "mile", "miles"], Tipo::Comprimento, 1609.344),
    (&["mg", "miligrama", "miligramas"], Tipo::Massa, 0.001),
    (&["g", "grama", "gramas"], Tipo::Massa, 1.0),
    (&["kg", "quilo", "quilos", "quilograma", "quilogramas"], Tipo::Massa, 1000.0),
    (&["t", "tonelada", "toneladas"], Tipo::Massa, 1_000_000.0),
    (&["lb", "lbs", "libra", "libras", "pound", "pounds"], Tipo::Massa, 453.59237),
    (&["oz", "onca", "oncas", "ounce", "ounces"], Tipo::Massa, 28.349523125),
    (&["b", "byte", "bytes"], Tipo::Dados, 1.0),
    (&["kb"], Tipo::Dados, 1e3),
    (&["mb"], Tipo::Dados, 1e6),
    (&["gb"], Tipo::Dados, 1e9),
    (&["tb"], Tipo::Dados, 1e12),
    (&["kib"], Tipo::Dados, 1024.0),
    (&["mib"], Tipo::Dados, 1048576.0),
    (&["gib"], Tipo::Dados, 1073741824.0),
    (&["tib"], Tipo::Dados, 1099511627776.0),
    (&["ms", "milissegundo", "milissegundos"], Tipo::Tempo, 0.001),
    (&["s", "seg", "segundo", "segundos"], Tipo::Tempo, 1.0),
    (&["min", "minuto", "minutos"], Tipo::Tempo, 60.0),
    (&["h", "hora", "horas"], Tipo::Tempo, 3600.0),
    (&["dia", "dias", "day", "days"], Tipo::Tempo, 86400.0),
    (&["semana", "semanas", "week", "weeks"], Tipo::Tempo, 604800.0),
    (&["ano", "anos", "year", "years"], Tipo::Tempo, 31_557_600.0),
    (&["ml", "mililitro", "mililitros"], Tipo::Volume, 0.001),
    (&["l", "litro", "litros"], Tipo::Volume, 1.0),
    (&["m3"], Tipo::Volume, 1000.0),
    (&["gal", "galao", "galoes", "gallon", "gallons"], Tipo::Volume, 3.785411784),
    (&["xicara", "xicaras", "cup", "cups"], Tipo::Volume, 0.24),
    (&["km/h", "kmh"], Tipo::Velocidade, 1.0 / 3.6),
    (&["m/s"], Tipo::Velocidade, 1.0),
    (&["mph"], Tipo::Velocidade, 0.44704),
    (&["no", "nos", "knot", "knots"], Tipo::Velocidade, 0.514444),
    (&["m2"], Tipo::Area, 1.0),
    (&["km2"], Tipo::Area, 1e6),
    (&["ha", "hectare", "hectares"], Tipo::Area, 10000.0),
    (&["alqueire", "alqueires"], Tipo::Area, 24200.0), // alqueire paulista
    (&["c", "celsius", "°c"], Tipo::Temperatura, 0.0),
    (&["f", "fahrenheit", "°f"], Tipo::Temperatura, 0.0),
    (&["k", "kelvin"], Tipo::Temperatura, 0.0),
];

/// Nomes de moeda que não são o código: "10 reais em dolar".
const MOEDAS: &[(&str, &str)] = &[
    ("real", "BRL"),
    ("reais", "BRL"),
    ("dolar", "USD"),
    ("dolares", "USD"),
    ("euro", "EUR"),
    ("euros", "EUR"),
    ("libra esterlina", "GBP"),
    ("iene", "JPY"),
    ("ienes", "JPY"),
    ("peso", "ARS"),
    ("pesos", "ARS"),
    ("bitcoin", "BTC"),
];

fn unidade(nome: &str) -> Option<(Tipo, f64, &'static str)> {
    UNIDADES.iter().find(|(nomes, ..)| nomes.contains(&nome)).map(|(nomes, t, f)| (*t, *f, nomes[0]))
}

fn para_kelvin(v: f64, u: &str) -> f64 {
    match u {
        "c" => v + 273.15,
        "f" => (v - 32.0) * 5.0 / 9.0 + 273.15,
        _ => v,
    }
}

fn de_kelvin(k: f64, u: &str) -> f64 {
    match u {
        "c" => k - 273.15,
        "f" => (k - 273.15) * 9.0 / 5.0 + 32.0,
        _ => k,
    }
}

/// "10,5 km em milhas" -> (10.5, "km", "milhas")
fn separar(texto: &str) -> Option<(f64, String, String)> {
    let t = matcher::norm(texto.trim()).replace(',', ".");
    let (esq, dir) = [" em ", " para ", " pra ", " in ", " to ", " -> ", " = "].iter().find_map(|s| t.split_once(s))?;
    let esq = esq.trim();
    // o número pode vir colado na unidade: "10km"
    let fim = esq.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).unwrap_or(esq.len());
    let valor: f64 = esq[..fim].trim().parse().ok()?;
    let de = esq[fim..].trim().to_string();
    let para = dir.trim().to_string();
    (!de.is_empty() && !para.is_empty()).then_some((valor, de, para))
}

fn moeda(nome: &str) -> Option<String> {
    if let Some((_, c)) = MOEDAS.iter().find(|(n, _)| *n == nome) {
        return Some(c.to_string());
    }
    (nome.len() == 3 && nome.chars().all(|c| c.is_ascii_alphabetic())).then(|| nome.to_uppercase())
}

fn agora() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn arquivo_cambio() -> std::path::PathBuf {
    pasta_estado().join("cambio.json")
}

fn baixar_cambio() -> Option<Cambio> {
    let texto = sistema::texto(&["curl", "-s", "--max-time", "4", "https://open.er-api.com/v6/latest/USD"], Duration::from_secs(5));
    let v: serde_json::Value = serde_json::from_str(&texto).ok()?;
    let taxas: HashMap<String, f64> = v.get("rates")?.as_object()?.iter().filter_map(|(k, x)| Some((k.clone(), x.as_f64()?))).collect();
    let c = Cambio { taxas, quando: agora() };
    let _ = std::fs::create_dir_all(pasta_estado());
    let _ = std::fs::write(arquivo_cambio(), serde_json::to_string(&c).unwrap_or_default());
    Some(c)
}

impl Conversor {
    pub fn new() -> Conversor {
        let c = std::fs::read_to_string(arquivo_cambio()).ok().and_then(|t| serde_json::from_str(&t).ok());
        Conversor { cambio: Mutex::new(c) }
    }

    /// Cotação guardada; se tiver mais de 12 horas, busca de novo (se falhar, usa a velha).
    fn cambio(&self) -> Option<Cambio> {
        let mut c = self.cambio.lock().unwrap();
        if c.as_ref().is_none_or(|x| agora() - x.quando > 12 * 3600) {
            if let Some(novo) = baixar_cambio() {
                *c = Some(novo);
            }
        }
        c.clone()
    }

    fn converter(&self, valor: f64, de: &str, para: &str) -> Option<(f64, String, String, String)> {
        if let (Some((t1, f1, n1)), Some((t2, f2, n2))) = (unidade(de), unidade(para)) {
            if t1 != t2 {
                return None;
            }
            let r = if t1 == Tipo::Temperatura { de_kelvin(para_kelvin(valor, n1), n2) } else { valor * f1 / f2 };
            return Some((r, n1.to_string(), n2.to_string(), String::new()));
        }
        let (m1, m2) = (moeda(de)?, moeda(para)?);
        let c = self.cambio()?;
        let (t1, t2) = (c.taxas.get(&m1)?, c.taxas.get(&m2)?);
        let dias = (agora() - c.quando) / 86400;
        let quando = if dias == 0 { "cotação de hoje".to_string() } else { format!("cotação de {dias} dia(s) atrás") };
        Some((valor / t1 * t2, m1, m2, quando))
    }
}

/// Número legível: 2 casas pra dinheiro e valores grandes, mais casas quando é pequeno.
fn bonito(v: f64, dinheiro: bool) -> String {
    let casas = if dinheiro || v.abs() >= 100.0 { 2 } else if v.abs() >= 1.0 { 4 } else { 6 };
    let fator = 10f64.powi(casas);
    formatar((v * fator).round() / fator)
}

impl Provider for Conversor {
    fn category(&self) -> &str {
        "Conversor"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let Some((valor, de, para)) = separar(&q.text) else { return vec![] };
        let Some((r, n1, n2, nota_cambio)) = self.converter(valor, &de, &para) else { return vec![] };
        let dinheiro = !nota_cambio.is_empty();
        let texto = bonito(r, dinheiro);
        let mut sub = format!("{} {n1} em {n2}", formatar(valor));
        if dinheiro {
            sub += &format!("  ·  {nota_cambio}");
        }
        vec![
            Item::new(format!("= {texto} {n2}"), Effect::Copy { text: texto.replace(' ', "") })
                .subtitle(format!("{sub}  ·  Enter copia"))
                .icon(if dinheiro { "money-management" } else { "accessories-calculator" })
                .score(1.0),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unidades() {
        let c = Conversor { cambio: Mutex::new(None) };
        let conv = |t: &str| {
            let (v, de, para) = separar(t).unwrap();
            c.converter(v, &de, &para).map(|(r, ..)| bonito(r, false))
        };
        assert_eq!(conv("5 km em milhas").as_deref(), Some("3.1069"));
        assert_eq!(conv("100 f em c").as_deref(), Some("37.7778"));
        assert_eq!(conv("2 gb em mb").as_deref(), Some("2000"));
        assert_eq!(conv("1,5 h em min").as_deref(), Some("90"));
        assert_eq!(conv("10km em m").as_deref(), Some("10 000"));
        assert_eq!(conv("5 km em kg"), None);
    }

    #[test]
    fn moedas() {
        let c = Conversor { cambio: Mutex::new(Some(Cambio { taxas: HashMap::from([("USD".into(), 1.0), ("BRL".into(), 5.0)]), quando: agora() })) };
        let (v, de, para) = separar("10 dolares em reais").unwrap();
        assert_eq!(c.converter(v, &de, &para).map(|(r, ..)| r), Some(50.0));
        let (v, de, para) = separar("10 usd em brl").unwrap();
        assert_eq!(c.converter(v, &de, &para).map(|(r, ..)| r), Some(50.0));
    }
}
