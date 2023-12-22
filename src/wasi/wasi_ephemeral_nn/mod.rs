use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::RwLock};

use crate::{module::ExternalFuncInst, Importer, Store, Value};
use anyhow::{Context as _, Result};

type GraphId = i32;
type RawModelData = Vec<u8>;

pub enum Backend {
    OpenVINO = 0,
    ONNX = 1,
    Tensorflow = 2,
    PyTorch = 3,
    TensorflowLite = 4,
    Autodetect = 5,
    GGML = 6,
}

impl From<String> for Backend {
    fn from(s: String) -> Self {
        match s.as_str() {
            "openvino" => Backend::OpenVINO,
            "onnx" => Backend::ONNX,
            "tensorflow" => Backend::Tensorflow,
            "pytorch" => Backend::PyTorch,
            "tensorflowlite" => Backend::TensorflowLite,
            "autodetect" => Backend::Autodetect,
            "ggml" => Backend::GGML,
            _ => unreachable!(),
        }
    }
}

pub enum Device {
    CPU = 0,
    GPU = 1,
    TPU = 2,
    AUTO = 3,
}

impl From<String> for Device {
    fn from(s: String) -> Self {
        match s.as_str() {
            "cpu" => Device::CPU,
            "gpu" => Device::GPU,
            "tpu" => Device::TPU,
            "auto" => Device::AUTO,
            _ => unreachable!(),
        }
    }
}

pub struct Graph {}

pub struct Context {}

pub struct WasiEphemeralNn {
    pub model_path: String,
    pub model_map: RwLock<HashMap<String, GraphId>>,
    pub raw_model_map: RwLock<HashMap<String, (RawModelData, Backend, Device)>>,
    pub nn_graph: Vec<Graph>,
    pub nn_context: Vec<Context>,
}

impl Importer for WasiEphemeralNn {
    fn name(&self) -> &str {
        "wasi_ephemeral_nn"
    }

    fn invoke(
        &self,
        store: Rc<RefCell<Store>>,
        func: ExternalFuncInst,
        args: Vec<Value>,
    ) -> Result<Option<Value>> {
        let value = match func.field.as_str() {
            "load_by_name" => self.load_by_name(store, args),
            "init_execution_context" => self.init_execution_context(store, args),
            "compute" => self.compute(store, args),
            "get_output" => self.get_output(store, args),
            "set_input" => self.set_input(store, args),
            _ => unreachable!(),
        }?;
        Ok(Some(value.into()))
    }
}

impl WasiEphemeralNn {
    // ref: https://github.com/WasmEdge/WasmEdge/blob/4f16952b3a1b45720e48230713306603b7f6da6e/plugins/wasi_nn/wasinnenv.h#L145-L202
    pub fn new(model_path: &str) -> Self {
        let mut iter = model_path.split(':');

        let Some(name) = iter.next() else {
            panic!("please specify model name, e.g. default:GGML:AUTO:llama-2-7b-chat-q5_k_m.gguf");
        };

        let Some(backend) = iter.next() else {
            panic!(
                "please specify model backend, e.g. default:GGML:AUTO:llama-2-7b-chat-q5_k_m.gguf"
            );
        };

        if backend != "GGML" {
            panic!("only support GGML backend");
        }

        let Some(device) = iter.next() else {
            panic!(
                "please specify model target, e.g. default:GGML:AUTO:llama-2-7b-chat-q5_k_m.gguf"
            );
        };

        let Some(path) = iter.next() else {
            panic!("please specify model path, e.g. default:GGML:AUTO:llama-2-7b-chat-q5_k_m.gguf");
        };

        let mut models = vec![];
        models.extend_from_slice(format!("preload:{}", path).as_bytes());

        let raw_model_map = RwLock::new(HashMap::from_iter([(
            name.into(),
            (
                models,
                backend.to_lowercase().into(),
                device.to_lowercase().into(),
            ),
        )]));

        Self {
            model_path: model_path.to_string(),
            model_map: RwLock::new(HashMap::new()),
            raw_model_map,
            nn_graph: vec![],
            nn_context: vec![],
        }
    }
}

// 1. create graph from model data.
//   1. preload model data
//   2. create graph
//   3. save graph id to model map
//     - key: model name
//     - value: graph id
// 2. get graph id by name using load_by_name(name_ptr, name_len, graph_id_ptr)
// 3.
impl WasiEphemeralNn {
    // ref: https://github.com/WasmEdge/WasmEdge/blob/ea977d278c9ce53898bad97d4ecaf9103234857d/plugins/wasi_nn/ggml.cpp#L175-L259
    fn load(&self, _builders: Vec<Vec<u8>>) -> GraphId {
        println!("load");
        0
    }

    // ref: https://github.com/WasmEdge/WasmEdge/blob/4f16952b3a1b45720e48230713306603b7f6da6e/plugins/wasi_nn/wasinnenv.h#L166-L190
    pub fn build_model(&self, model_name: &str) -> GraphId {
        println!("build model");
        let raw_model_map = self.raw_model_map.read().unwrap();
        let (model, _, _) = raw_model_map.get(model_name).unwrap();

        // ref: https://github.com/second-state/WasmEdge-WASINN-examples/blob/759147c843662fab66444f5a8d402b7eb68d6b4e/wasmedge-ggml-llama-interactive/src/main.rs#L60-L66C8
        let builders = vec![model.clone()];

        // load model
        self.load(builders)
    }

    // get graph id by name
    // ref: https://github.com/WasmEdge/WasmEdge/blob/4f16952b3a1b45720e48230713306603b7f6da6e/plugins/wasi_nn/wasinnfunc.cpp#L94-L123
    pub fn load_by_name(&self, store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<i32> {
        println!("load_by_name");
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (name_ptr, name_len, graph_id_ptr) =
            (args[0] as usize, args[1] as usize, args[2] as usize);

        let store = store.borrow();
        let memory = store.memory.get(0).with_context(|| "not found memory")?;
        let mut memory = memory.borrow_mut();

        let buf = &memory.data[name_ptr..name_ptr + name_len];
        let model_name = String::from_utf8(buf.to_vec()).unwrap();

        let graph_id = match self.model_map.read().unwrap().get(&model_name) {
            Some(graph_id) => *graph_id,
            None => self.build_model(&model_name),
        };

        memory.write_bytes(graph_id_ptr, &graph_id.to_be_bytes())?;

        Ok(0)
    }

    pub fn init_execution_context(
        &self,
        _store: Rc<RefCell<Store>>,
        args: Vec<Value>,
    ) -> Result<i32> {
        println!("init_execution_context");
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (graph_handle, ctx_ptr) = (args[0] as usize, args[1] as usize);
        dbg!(graph_handle, ctx_ptr);
        Ok(0)
    }

    pub fn compute(&self, _store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<i32> {
        println!("compute");
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let ctx_compute = args[0];
        dbg!(ctx_compute);
        Ok(0)
    }

    pub fn get_output(&self, _store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<i32> {
        println!("get_output");
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (ctx_ptr, index, out_buf_ptr, out_buf_len, out_size) = (
            args[0] as usize,
            args[1] as usize,
            args[2] as usize,
            args[3] as usize,
            args[4] as usize,
        );

        dbg!(ctx_ptr, index, out_buf_ptr, out_buf_len, out_size);
        Ok(0)
    }

    pub fn set_input(&self, _store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<i32> {
        println!("set_input");
        let args: Vec<i32> = args.into_iter().map(Into::into).collect();
        let (ctx_ptr, index, tensor) = (args[0] as usize, args[1] as usize, args[2] as usize);
        dbg!(ctx_ptr, index, tensor);
        Ok(0)
    }
}
