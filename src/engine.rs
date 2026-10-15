//! Manda a busca pra todos os provedores ao mesmo tempo e junta as respostas por categoria.

use crate::config::Config;
use crate::matcher;
use crate::model::{Effect, Item};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};

static ATUALIZAR: OnceLock<async_channel::Sender<()>> = OnceLock::new();

/// true enquanto a janela está aberta (um provedor pode avisar por notificação quando não está).
pub static VISIVEL: AtomicBool = AtomicBool::new(false);

/// Pede pra janela refazer a busca atual (a resposta da IA chegando aos poucos, por exemplo).
/// Pode ser chamado de qualquer thread.
pub fn pedir_atualizacao() {
    if let Some(tx) = ATUALIZAR.get() {
        let _ = tx.try_send(());
    }
}

pub fn ligar_atualizacao(tx: async_channel::Sender<()>) {
    let _ = ATUALIZAR.set(tx);
}

static COPIAR: OnceLock<async_channel::Sender<String>> = OnceLock::new();

/// Copia pra área de transferência sem fechar a janela. Pode ser chamado de qualquer thread.
pub fn copiar(texto: String) {
    if let Some(tx) = COPIAR.get() {
        let _ = tx.try_send(texto);
    }
}

pub fn ligar_copiar(tx: async_channel::Sender<String>) {
    let _ = COPIAR.set(tx);
}

pub struct Query {
    /// O texto como foi digitado (sem a palavra-chave, se tinha).
    pub text: String,
    /// Normalizado: minúsculo, sem acento.
    pub norm: String,
    /// true quando a busca começou com a palavra-chave deste provedor ("vs poke").
    pub with_keyword: bool,
}

pub trait Provider: Send + Sync {
    /// Nome da categoria, que aparece na coluna da esquerda ("Aplicativos").
    fn category(&self) -> &str;
    /// Palavra-chave que filtra só este provedor ("vs"); vazio = não tem.
    fn keyword(&self) -> &str {
        ""
    }
    /// true se só responde com a palavra-chave.
    fn needs_keyword(&self) -> bool {
        false
    }
    /// Roda numa thread própria; pode demorar (o resto da lista não espera).
    fn query(&self, q: &Query) -> Vec<Item>;
    /// Resolve um Effect::Callback que este provedor criou. Roda numa thread. Lista vazia fecha
    /// a janela; pra deixar aberta, devolva [Effect::None].
    fn callback(&self, _data: &str) -> Vec<Effect> {
        vec![]
    }
}

/// Resposta de um provedor pra uma busca.
pub struct Batch {
    pub generation: u64,
    pub provider: usize,
    pub items: Vec<Item>,
}

pub struct Engine {
    pub providers: Vec<Arc<dyn Provider>>,
}

impl Engine {
    /// Dispara a busca. Cada provedor responde no `tx` quando terminar.
    pub fn search(&self, text: &str, generation: u64, tx: &async_channel::Sender<Batch>, cfg: &Config) {
        let (palavra, resto) = match text.split_once(' ') {
            Some((p, r)) => (p, r.trim_start()),
            None => (text, ""),
        };
        let dono = self.providers.iter().position(|p| !p.keyword().is_empty() && p.keyword() == palavra);
        for (i, p) in self.providers.iter().enumerate() {
            if cfg.categorias.esconder.iter().any(|c| c == p.category()) {
                continue;
            }
            let q = match dono {
                Some(d) if d == i => Query { text: resto.to_string(), norm: matcher::norm(resto), with_keyword: true },
                Some(_) => continue, // "vs poke": só o vs responde
                None if p.needs_keyword() => continue,
                None => Query { text: text.to_string(), norm: matcher::norm(text.trim()), with_keyword: false },
            };
            let p = p.clone();
            let tx = tx.clone();
            std::thread::spawn(move || {
                let items = p.query(&q);
                let _ = tx.send_blocking(Batch { generation, provider: i, items });
            });
        }
    }
}

/// Uma linha da lista já montada.
pub struct Row {
    pub provider: usize,
    pub category: String,
    pub first_of_group: bool,
    pub item: Item,
}

/// Junta as respostas: categorias ordenadas pelo melhor resultado de cada uma (desempate pela
/// ordem da config), cada uma com até `limite` itens. Com palavra-chave não tem limite.
pub fn arrange(engine: &Engine, results: &[Option<Vec<Item>>], cfg: &Config, keyword_mode: bool) -> Vec<Row> {
    struct Grupo<'a> {
        categoria: &'a str,
        melhor: f32,
        ordem: usize,
        itens: Vec<(usize, &'a Item)>,
    }
    let mut grupos: Vec<Grupo> = Vec::new();
    for (i, r) in results.iter().enumerate() {
        let Some(itens) = r else { continue };
        for it in itens {
            let categoria = it.category.as_deref().unwrap_or(engine.providers[i].category());
            if cfg.categorias.esconder.iter().any(|c| c == categoria) {
                continue;
            }
            let g = match grupos.iter().position(|g| g.categoria == categoria) {
                Some(g) => &mut grupos[g],
                None => {
                    let ordem = cfg.categorias.ordem.iter().position(|c| c == categoria).unwrap_or(usize::MAX);
                    grupos.push(Grupo { categoria, melhor: 0.0, ordem, itens: vec![] });
                    grupos.last_mut().unwrap()
                }
            };
            g.melhor = g.melhor.max(it.score);
            g.itens.push((i, it));
        }
    }
    // Notas parecidas contam como empate, senão a ordem da config nunca vale
    let faixa = |s: f32| (s * 10.0).round() as i32;
    grupos.sort_by(|a, b| faixa(b.melhor).cmp(&faixa(a.melhor)).then(a.ordem.cmp(&b.ordem)));
    let mut rows = Vec::new();
    for mut g in grupos {
        g.itens.sort_by(|a, b| b.1.score.total_cmp(&a.1.score));
        // A mesma coisa vinda de dois provedores (a pasta que o Baloo também achou) aparece uma vez
        let mut vistos = std::collections::HashSet::new();
        g.itens.retain(|(_, it)| vistos.insert((it.title.clone(), it.subtitle.clone())));
        let limite = if keyword_mode { usize::MAX } else { cfg.limite_de(g.categoria) };
        for (n, (i, it)) in g.itens.into_iter().take(limite).enumerate() {
            rows.push(Row { provider: i, category: g.categoria.to_string(), first_of_group: n == 0, item: it.clone() });
        }
    }
    rows
}
