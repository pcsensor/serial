use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::services::store::{Encoding, PresetCommand};

const PRESET_COMMANDS_STORE_PATH: &str = "preset-commands.json";
const PRESET_COMMANDS_KEY: &str = "preset_commands";

fn log_err(msg: &str) {
    web_sys::console::error_1(&msg.into());
}

fn get_tauri_core_invoke() -> Result<js_sys::Function, String> {
    let window = web_sys::window().ok_or("无法获取 window 对象")?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|_| "无法读取 window.__TAURI__")?;
    if tauri.is_undefined() {
        return Err("window.__TAURI__ 未定义，Tauri 初始化脚本可能未加载".into());
    }
    let core =
        js_sys::Reflect::get(&tauri, &"core".into()).map_err(|_| "无法读取 __TAURI__.core")?;
    let invoke_fn = js_sys::Reflect::get(&core, &"invoke".into())
        .map_err(|_| "无法读取 __TAURI__.core.invoke")?;
    invoke_fn
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "__TAURI__.core.invoke 不是一个函数".into())
}

fn get_tauri_event_listen() -> Result<js_sys::Function, String> {
    let window = web_sys::window().ok_or("无法获取 window 对象")?;
    let tauri = js_sys::Reflect::get(&window, &"__TAURI__".into())
        .map_err(|_| "无法读取 window.__TAURI__")?;
    let event =
        js_sys::Reflect::get(&tauri, &"event".into()).map_err(|_| "无法读取 __TAURI__.event")?;
    let listen_fn = js_sys::Reflect::get(&event, &"listen".into())
        .map_err(|_| "无法读取 __TAURI__.event.listen")?;
    listen_fn
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "__TAURI__.event.listen 不是一个函数".into())
}

pub async fn tauri_invoke<T: Serialize, R: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &T,
) -> Result<R, String> {
    let invoke_fn = get_tauri_core_invoke()?;
    let args_js = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    let args_array = js_sys::Array::new();
    args_array.push(&JsValue::from(cmd));
    args_array.push(&args_js);
    let promise = invoke_fn
        .apply(&JsValue::UNDEFINED, &args_array)
        .map_err(|e| format!("invoke(\"{}\") 调用失败: {:?}", cmd, e))?;
    let result = JsFuture::from(js_sys::Promise::from(promise))
        .await
        .map_err(|e| format!("invoke(\"{}\") Promise 拒绝: {:?}", cmd, e))?;
    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("invoke(\"{}\") 返回值解析失败: {}", cmd, e))
}

pub async fn tauri_invoke_no_args<R: for<'de> Deserialize<'de>>(cmd: &str) -> Result<R, String> {
    let invoke_fn = get_tauri_core_invoke()?;
    let args_array = js_sys::Array::new();
    args_array.push(&JsValue::from(cmd));
    let promise = invoke_fn
        .apply(&JsValue::UNDEFINED, &args_array)
        .map_err(|e| format!("invoke(\"{}\") 调用失败: {:?}", cmd, e))?;
    let result = JsFuture::from(js_sys::Promise::from(promise))
        .await
        .map_err(|e| format!("invoke(\"{}\") Promise 拒绝: {:?}", cmd, e))?;
    serde_wasm_bindgen::from_value(result)
        .map_err(|e| format!("invoke(\"{}\") 返回值解析失败: {}", cmd, e))
}

#[derive(Serialize)]
struct SendDataArgs {
    request: SendDataRequest,
}

#[derive(Serialize)]
struct SendDataRequest {
    content: String,
    encoding: Encoding,
}

pub async fn send_serial_data(content: String, encoding: Encoding) -> Result<usize, String> {
    tauri_invoke(
        "send_data",
        &SendDataArgs {
            request: SendDataRequest { content, encoding },
        },
    )
    .await
}

#[derive(Serialize)]
struct StoreLoadArgs<'a> {
    path: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct StoreKeyArgs<'a> {
    rid: u32,
    key: &'a str,
}

#[derive(Serialize)]
struct StoreSetArgs<'a> {
    rid: u32,
    key: &'a str,
    value: serde_json::Value,
}

#[derive(Serialize)]
struct StoreResourceArgs {
    rid: u32,
}

fn preset_store_load_args() -> StoreLoadArgs<'static> {
    StoreLoadArgs {
        path: PRESET_COMMANDS_STORE_PATH,
        options: None,
    }
}

fn preset_store_get_args(rid: u32) -> StoreKeyArgs<'static> {
    StoreKeyArgs {
        rid,
        key: PRESET_COMMANDS_KEY,
    }
}

fn preset_store_set_args(
    rid: u32,
    commands: &[PresetCommand],
) -> Result<StoreSetArgs<'static>, String> {
    Ok(StoreSetArgs {
        rid,
        key: PRESET_COMMANDS_KEY,
        value: serde_json::to_value(commands).map_err(|e| e.to_string())?,
    })
}

fn store_resource_args(rid: u32) -> StoreResourceArgs {
    StoreResourceArgs { rid }
}

async fn load_preset_store_resource() -> Result<u32, String> {
    tauri_invoke("plugin:store|load", &preset_store_load_args()).await
}

pub async fn load_preset_commands() -> Result<Vec<PresetCommand>, String> {
    let rid = load_preset_store_resource().await?;
    let (value, exists): (Option<serde_json::Value>, bool) =
        tauri_invoke("plugin:store|get", &preset_store_get_args(rid)).await?;

    if !exists {
        return Ok(Vec::new());
    }

    serde_json::from_value(value.unwrap_or(serde_json::Value::Null))
        .map_err(|e| format!("解析预设指令失败: {}", e))
}

pub async fn save_preset_commands(commands: &[PresetCommand]) -> Result<(), String> {
    let rid = load_preset_store_resource().await?;
    let args = preset_store_set_args(rid, commands)?;
    tauri_invoke::<_, ()>("plugin:store|set", &args).await?;
    tauri_invoke::<_, ()>("plugin:store|save", &store_resource_args(rid)).await
}

pub fn tauri_listen<F>(event: &str, callback: F)
where
    F: 'static + FnMut(JsValue),
{
    let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(JsValue)>);
    match get_tauri_event_listen() {
        Ok(listen_fn) => {
            let args_array = js_sys::Array::new();
            args_array.push(&JsValue::from(event));
            args_array.push(closure.as_ref());
            let _ = listen_fn.apply(&JsValue::UNDEFINED, &args_array);
            closure.forget();
        }
        Err(e) => {
            log_err(&format!("tauri_listen(\"{}\") 失败: {}", event, e));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preset_store_get_args_include_loaded_resource_id() {
        let args = preset_store_get_args(42);

        assert_eq!(
            serde_json::to_value(args).unwrap(),
            json!({
                "rid": 42,
                "key": "preset_commands"
            })
        );
    }

    #[test]
    fn preset_store_save_args_include_loaded_resource_id() {
        let args = store_resource_args(42);

        assert_eq!(
            serde_json::to_value(args).unwrap(),
            json!({
                "rid": 42
            })
        );
    }
}
