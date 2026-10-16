//! Janela no topo da tela pelo gtk4-layer-shell, carregado na hora de abrir (não na compilação).
//! Assim, quem instala o gtk4-layer-shell depois do jacundash não precisa recompilar: vale no
//! próximo início (ou depois de um `jacundash --sair`).
//!
//! O gtk4-layer-shell precisa ser carregado antes do libwayland-client, então quando ele existe
//! o jacundash se reabre uma vez com LD_PRELOAD. Os apps abertos por ele não herdam isso.

use gtk::glib::translate::ToGlibPtr;
use gtk::prelude::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::process::CommandExt;
use std::sync::OnceLock;

const LIB: &str = "libgtk4-layer-shell.so.0";
const LIB_C: &std::ffi::CStr = c"libgtk4-layer-shell.so.0";
const RTLD_LAZY: c_int = 1;
const RTLD_NOLOAD: c_int = 4;
/// Guarda o LD_PRELOAD de antes, pra devolver depois do reinício (e não reiniciar em loop)
const ANTES: &str = "JACUNDASH_LD_PRELOAD";

// Valores dos enums do gtk4-layer-shell
const LAYER_OVERLAY: c_int = 3;
const EDGE_TOP: c_int = 2;
const TECLADO_SOB_DEMANDA: c_int = 2;

unsafe extern "C" {
    fn dlopen(nome: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(lib: *mut c_void, simbolo: *const c_char) -> *mut c_void;
}

fn carregada() -> *mut c_void {
    unsafe { dlopen(LIB_C.as_ptr(), RTLD_LAZY | RTLD_NOLOAD) }
}

/// Chamado no começo do main, antes do GTK. Se o gtk4-layer-shell está instalado e ainda não
/// foi carregado, reabre o jacundash com ele no LD_PRELOAD.
pub fn preparar() {
    if let Some(antes) = std::env::var_os(ANTES) {
        // Já é o reaberto: tira o preload do ambiente pros apps que ele abrir
        // SAFETY: começo do main, nenhuma outra thread ainda
        unsafe {
            if antes.is_empty() {
                std::env::remove_var("LD_PRELOAD");
            } else {
                std::env::set_var("LD_PRELOAD", &antes);
            }
            std::env::remove_var(ANTES);
        }
        return;
    }
    if std::env::var_os("WAYLAND_DISPLAY").is_none() || !carregada().is_null() {
        return;
    }
    // Já tem um rodando: este só manda o recado pelo D-Bus, não abre janela
    if crate::krunner::existe(crate::APP_ID) {
        return;
    }
    // Instalado? (carregar aqui não atrapalha: o exec joga este processo fora)
    if unsafe { dlopen(LIB_C.as_ptr(), RTLD_LAZY) }.is_null() {
        return;
    }
    let antes = std::env::var("LD_PRELOAD").unwrap_or_default();
    let preload = if antes.is_empty() { LIB.to_string() } else { format!("{LIB}:{antes}") };
    let mut args = std::env::args_os();
    let Some(prog) = args.next() else { return };
    let exe = std::env::current_exe().map(|p| p.into_os_string()).unwrap_or(prog);
    // Só volta se o exec falhar; aí segue como janela comum
    let _ = std::process::Command::new(exe).args(args).env("LD_PRELOAD", preload).env(ANTES, &antes).exec();
}

type Janela = *mut c_void;

struct Funcoes {
    init: unsafe extern "C" fn(Janela),
    namespace: unsafe extern "C" fn(Janela, *const c_char),
    layer: unsafe extern "C" fn(Janela, c_int),
    anchor: unsafe extern "C" fn(Janela, c_int, c_int),
    teclado: unsafe extern "C" fn(Janela, c_int),
    margem: unsafe extern "C" fn(Janela, c_int, c_int),
}

/// As funções do gtk4-layer-shell, se ele foi carregado e o compositor suporta (o GNOME não).
fn funcoes() -> Option<&'static Funcoes> {
    static F: OnceLock<Option<Funcoes>> = OnceLock::new();
    F.get_or_init(|| {
        let lib = carregada();
        if lib.is_null() {
            return None;
        }
        let s = |nome: &std::ffi::CStr| Some(unsafe { dlsym(lib, nome.as_ptr()) }).filter(|p| !p.is_null());
        // SAFETY: as assinaturas são as do gtk4-layer-shell 1.x
        unsafe {
            let suportado: unsafe extern "C" fn() -> c_int = std::mem::transmute(s(c"gtk_layer_is_supported")?);
            if suportado() == 0 {
                return None;
            }
            Some(Funcoes {
                init: std::mem::transmute(s(c"gtk_layer_init_for_window")?),
                namespace: std::mem::transmute(s(c"gtk_layer_set_namespace")?),
                layer: std::mem::transmute(s(c"gtk_layer_set_layer")?),
                anchor: std::mem::transmute(s(c"gtk_layer_set_anchor")?),
                teclado: std::mem::transmute(s(c"gtk_layer_set_keyboard_mode")?),
                margem: std::mem::transmute(s(c"gtk_layer_set_margin")?),
            })
        }
    })
    .as_ref()
}

fn ptr(window: &impl IsA<gtk::Window>) -> Janela {
    let p: *mut gtk::ffi::GtkWindow = window.upcast_ref::<gtk::Window>().to_glib_none().0;
    p.cast()
}

/// Cola a janela no topo da tela, por cima de tudo. Falso se não deu (vira janela comum).
pub fn iniciar(window: &impl IsA<gtk::Window>) -> bool {
    let Some(f) = funcoes() else { return false };
    let w = ptr(window);
    unsafe {
        (f.init)(w);
        (f.namespace)(w, c"jacundash".as_ptr());
        (f.layer)(w, LAYER_OVERLAY);
        (f.anchor)(w, EDGE_TOP, 1);
        (f.teclado)(w, TECLADO_SOB_DEMANDA);
    }
    true
}

pub fn margem_topo(window: &impl IsA<gtk::Window>, px: i32) {
    if let Some(f) = funcoes() {
        unsafe { (f.margem)(ptr(window), EDGE_TOP, px) }
    }
}
