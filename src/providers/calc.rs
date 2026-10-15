//! Calculadora: "2^10*3", "15% de 80", "sqrt(2)", "=1/3". Enter copia o resultado.

use crate::engine::{Provider, Query};
use crate::model::{Effect, Item};

pub struct Calc;

impl Provider for Calc {
    fn category(&self) -> &str {
        "Calculadora"
    }
    fn query(&self, q: &Query) -> Vec<Item> {
        let texto = q.text.trim();
        let conta = texto.strip_prefix('=').unwrap_or(texto);
        // Só tenta quando parece conta: tem número e (operador, função ou o "=" na frente)
        let tem_operador = conta.chars().any(|c| "+-*/^%()".contains(c)) || conta.contains(" de ");
        if !conta.chars().any(|c| c.is_ascii_digit()) || !(tem_operador || texto.starts_with('=')) {
            return vec![];
        }
        let Some(valor) = calcular(conta) else { return vec![] };
        let texto_valor = formatar(valor);
        vec![
            Item::new(format!("= {texto_valor}"), Effect::Copy { text: texto_valor.replace(' ', "") })
                .subtitle(format!("{conta}  ·  Enter copia"))
                .icon("accessories-calculator")
                .score(1.0),
        ]
    }
}

pub fn formatar(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        let s = format!("{}", v as i64);
        if v.abs() >= 10000.0 {
            // 1234567 -> 1 234 567
            let (sinal, dig) = if let Some(d) = s.strip_prefix('-') { ("-", d) } else { ("", s.as_str()) };
            let mut out = String::new();
            for (i, c) in dig.chars().enumerate() {
                if i > 0 && (dig.len() - i) % 3 == 0 {
                    out.push(' ');
                }
                out.push(c);
            }
            return format!("{sinal}{out}");
        }
        return s;
    }
    let s = format!("{:.10}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Avalia a expressão. None se não for uma conta válida.
pub fn calcular(expr: &str) -> Option<f64> {
    let expr = expr.replace(',', ".").replace('×', "*").replace('÷', "/").to_lowercase();
    let tokens = tokenizar(&expr)?;
    let mut p = Parser { t: tokens, i: 0 };
    let v = p.soma()?;
    (p.i == p.t.len() && v.is_finite()).then_some(v)
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Op(char),
    Nome(String),
    De, // "15% de 80"
}

fn tokenizar(s: &str) -> Option<Vec<Tok>> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() || ch == '.' {
            let ini = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            out.push(Tok::Num(c[ini..i].iter().collect::<String>().parse().ok()?));
        } else if ch.is_alphabetic() {
            let ini = i;
            while i < c.len() && c[i].is_alphanumeric() {
                i += 1;
            }
            let nome: String = c[ini..i].iter().collect();
            out.push(if nome == "de" || nome == "of" { Tok::De } else { Tok::Nome(nome) });
        } else if "+-*/^%()".contains(ch) {
            out.push(Tok::Op(ch));
            i += 1;
        } else {
            return None;
        }
    }
    Some(out)
}

struct Parser {
    t: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn ver(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn op(&mut self, c: char) -> bool {
        if self.ver() == Some(&Tok::Op(c)) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn soma(&mut self) -> Option<f64> {
        let mut v = self.produto()?;
        loop {
            if self.op('+') {
                v += self.produto()?;
            } else if self.op('-') {
                v -= self.produto()?;
            } else {
                return Some(v);
            }
        }
    }
    fn produto(&mut self) -> Option<f64> {
        let mut v = self.potencia()?;
        loop {
            if self.op('*') {
                v *= self.potencia()?;
            } else if self.op('/') {
                v /= self.potencia()?;
            } else if matches!(self.ver(), Some(Tok::Num(_)) | Some(Tok::Op('(')) | Some(Tok::Nome(_))) {
                v *= self.potencia()?; // 2pi, 3(4+1)
            } else {
                return Some(v);
            }
        }
    }
    fn potencia(&mut self) -> Option<f64> {
        let base = self.unario()?;
        if self.op('^') {
            return Some(base.powf(self.potencia()?)); // associa pela direita
        }
        Some(base)
    }
    fn unario(&mut self) -> Option<f64> {
        if self.op('-') {
            return Some(-self.unario()?);
        }
        if self.op('+') {
            return self.unario();
        }
        self.porcento()
    }
    fn porcento(&mut self) -> Option<f64> {
        let v = self.atomo()?;
        if self.op('%') {
            if self.ver() == Some(&Tok::De) {
                self.i += 1;
                return Some(v / 100.0 * self.unario()?);
            }
            return Some(v / 100.0);
        }
        Some(v)
    }
    fn atomo(&mut self) -> Option<f64> {
        match self.ver()?.clone() {
            Tok::Num(n) => {
                self.i += 1;
                Some(n)
            }
            Tok::Op('(') => {
                self.i += 1;
                let v = self.soma()?;
                self.op(')').then_some(v)
            }
            Tok::Nome(nome) => {
                self.i += 1;
                match nome.as_str() {
                    "pi" => return Some(std::f64::consts::PI),
                    "e" => return Some(std::f64::consts::E),
                    _ => {}
                }
                let x = self.unario()?;
                Some(match nome.as_str() {
                    "sqrt" | "raiz" => x.sqrt(),
                    "sin" | "sen" => x.sin(),
                    "cos" => x.cos(),
                    "tan" | "tg" => x.tan(),
                    "ln" => x.ln(),
                    "log" => x.log10(),
                    "abs" => x.abs(),
                    "exp" => x.exp(),
                    _ => return None,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contas() {
        assert_eq!(calcular("2^10*3"), Some(3072.0));
        assert_eq!(calcular("15% de 80"), Some(12.0));
        assert_eq!(calcular("2^3^2"), Some(512.0));
        assert_eq!(calcular("-(1+2)*3"), Some(-9.0));
        assert_eq!(calcular("1,5*2"), Some(3.0));
        assert_eq!(calcular("2pi").map(|v| (v * 1000.0).round()), Some(6283.0));
        assert_eq!(calcular("abc"), None);
        assert_eq!(calcular("1+"), None);
        assert_eq!(formatar(1234567.0), "1 234 567");
        assert_eq!(formatar(1.0 / 3.0), "0.3333333333");
    }
}
