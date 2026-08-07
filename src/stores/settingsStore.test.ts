import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { settingUpdaters } from "./settingsStore";

/**
 * Regression coverage for the formalizador de correo final review, hallazgo 1:
 * three new settings (user_full_name, formality_treatment,
 * formalize_prompt_id) had no entry in `settingUpdaters`. `updateSetting()`
 * updates the zustand state optimistically regardless, so the UI looked like
 * it worked, but with no updater the change silently never reached the Rust
 * backend (it fell through to the `console.warn` branch in
 * settingsStore.ts) and was lost on the next refresh or restart. Every email
 * came out using the stale backend defaults (tuteo, sin firma).
 *
 * VEKTRUN 2026-08-07 — ESTE TEST NO HACIA SU TRABAJO.
 *
 * Comprobaba esos tres campos POR NOMBRE. La clase de fallo que documenta es
 * "un ajuste nuevo se queda sin updater", y un test que nombra tres campos solo
 * detecta que esos tres siguen ahi — nunca el cuarto.
 *
 * Y volvio a pasar: `microphone_gain` no tenia updater ni comando Tauri. El
 * slider se movia, mostraba "2.5x", y las 189 lineas de `input_gain.rs` corrian
 * siempre a 1.0. Este test estaba verde mientras tanto.
 *
 * Ahora comprueba la INVARIANTE: todo campo de `AppSettings` tiene updater, o
 * esta en una lista de exclusion EXPLICITA con su motivo escrito. Anadir un
 * ajuste sin cablearlo pone el test en rojo; excluirlo obliga a justificarlo
 * aqui, que es donde alguien lo leera.
 *
 * La fuente de verdad es `bindings.ts`, que genera tauri-specta desde el Rust:
 * si el campo existe en `AppSettings`, existe en el backend.
 */

/**
 * Campos de `AppSettings` que legitimamente NO se escriben por
 * `updateSetting()`. Cada uno con el motivo: sin motivo, no entra.
 */
const SIN_UPDATER: Record<string, string> = {
  // Tienen su propia interfaz de edicion (no es un control de Ajustes que
  // escriba un valor suelto).
  bindings: "se editan con su propio dialogo de atajos",
  selected_model: "lo escribe el selector de modelo, con descarga previa",

  // Los escribe el backend, la interfaz solo los lee.
  settings_schema_version: "lo lleva la migracion de esquema, no el usuario",
  whats_new_last_seen_version:
    "lo sella la app al mostrar las novedades (change_whats_new_last_seen_version_setting)",
  onboarding_completed: "lo escribe el flujo de alta desde App.tsx, no Ajustes",

  // Tienen comando Tauri propio, verificado uno a uno el 2026-08-07.
  // No basta con que exista el campo: se comprobo que hay comando Y que hay
  // un componente que lo llama.
  model_unload_timeout:
    "set_model_unload_timeout, desde ModelUnloadTimeout.tsx",
  keyboard_implementation:
    "change_keyboard_implementation_setting, desde KeyboardImplementationSelector.tsx",
  post_process_provider_id:
    "lo lleva usePostProcessProviderState con su propio comando",
  post_process_models:
    "lo lleva usePostProcessProviderState al listar modelos del proveedor",

  // Colecciones con comandos propios de alta/baja/edicion.
  post_process_providers: "alta/baja por proveedor, no un valor suelto",
  post_process_api_keys: "comando propio por proveedor",
  post_process_prompts: "alta/baja de perfiles",
  custom_replacements: "el diccionario tiene sus propios comandos",
  custom_words: "lista con comandos propios",
  custom_filler_words: "lista con comandos propios",
};

/**
 * Saca los nombres de campo de `export type AppSettings = { ... }` de
 * bindings.ts, saltandose comentarios de bloque (que contienen `:` y `;` y
 * envenenarian un regex ingenuo).
 */
function camposDeAppSettings(): string[] {
  const ruta = join(import.meta.dir, "..", "bindings.ts");
  const fuente = readFileSync(ruta, "utf8");

  const marca = "export type AppSettings = {";
  const inicio = fuente.indexOf(marca);
  expect(inicio, "no se encontro AppSettings en bindings.ts").toBeGreaterThan(-1);

  // Cierre por conteo de llaves, no por "la primera linea que empieza por }":
  // AppSettings tiene tipos anidados en linea, y buscar el primer salto+llave
  // se colaba en los tipos SIGUIENTES del fichero y recogia sus campos.
  const desde = inicio + marca.length - 1; // sobre la '{' de apertura
  let profundidad = 0;
  let fin = -1;
  for (let i = desde; i < fuente.length; i++) {
    if (fuente[i] === "{") profundidad++;
    else if (fuente[i] === "}") {
      profundidad--;
      if (profundidad === 0) {
        fin = i;
        break;
      }
    }
  }
  expect(fin, "no se encontro el cierre de AppSettings").toBeGreaterThan(desde);

  // Fuera los comentarios de bloque antes de buscar campos.
  const cuerpo = fuente.slice(desde + 1, fin);
  const limpio = cuerpo.replace(/\/\*[\s\S]*?\*\//g, "");

  // Solo campos de PRIMER nivel: un tipo anidado en linea tiene sus propias
  // claves y no son ajustes.
  const campos = new Set<string>();
  let nivel = 0;
  for (let i = 0; i < limpio.length; i++) {
    const c = limpio[i];
    if (c === "{" || c === "[" || c === "(") nivel++;
    else if (c === "}" || c === "]" || c === ")") nivel--;
    else if (nivel === 0) {
      const m = /^([a-z_][a-z0-9_]*)\??\s*:/i.exec(limpio.slice(i));
      const previo = limpio[i - 1] ?? ";";
      if (m && /[;\s{]/.test(previo)) {
        campos.add(m[1]);
        i += m[0].length - 1;
      }
    }
  }
  return [...campos];
}

describe("settingUpdaters", () => {
  test("bindings.ts expone los campos de AppSettings", () => {
    // Si el parser deja de encontrar campos, el test de abajo pasaria en vacio
    // sin comprobar nada. Un cero que nadie mira es un verde falso.
    expect(camposDeAppSettings().length).toBeGreaterThan(20);
  });

  test("todo campo de AppSettings tiene updater o exclusion justificada", () => {
    const huerfanos = camposDeAppSettings().filter(
      (campo) =>
        !(campo in settingUpdaters) && !(campo in SIN_UPDATER),
    );

    expect(
      huerfanos,
      `Estos ajustes existen en el backend pero no se pueden escribir desde la ` +
        `interfaz: el control se movera y el valor no llegara nunca a Rust. ` +
        `Anade su entrada en settingUpdaters, o justifica la exclusion en ` +
        `SIN_UPDATER.\n  ${huerfanos.join("\n  ")}`,
    ).toEqual([]);
  });

  test("la lista de exclusion no acumula campos que ya no existen", () => {
    // Una exclusion caducada es peor que ninguna: tapa el hueco siguiente.
    const campos = new Set(camposDeAppSettings());
    const caducadas = Object.keys(SIN_UPDATER).filter((c) => !campos.has(c));

    expect(
      caducadas,
      `Estos campos estan excluidos en SIN_UPDATER pero ya no existen en ` +
        `AppSettings. Borralos: ${caducadas.join(", ")}`,
    ).toEqual([]);
  });

  test("ningun updater apunta a un campo inexistente", () => {
    const campos = new Set(camposDeAppSettings());
    const fantasmas = Object.keys(settingUpdaters).filter((c) => !campos.has(c));

    expect(
      fantasmas,
      `Estos updaters escriben campos que no estan en AppSettings: ` +
        `${fantasmas.join(", ")}`,
    ).toEqual([]);
  });

  test("los tres ajustes del formalizador siguen cableados", () => {
    // El caso original que motivo el fichero. Se conserva explicito.
    expect(settingUpdaters.user_full_name).toBeDefined();
    expect(settingUpdaters.formality_treatment).toBeDefined();
    expect(settingUpdaters.formalize_prompt_id).toBeDefined();
  });

  test("microphone_gain esta cableado", () => {
    // El campo que este fichero dejo pasar mientras estaba verde.
    expect(settingUpdaters.microphone_gain).toBeDefined();
  });
});
