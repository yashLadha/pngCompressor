use neon::prelude::*;

extern crate oxipng;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

struct CompressTask {
    input: String,
    out: String,
}

/// Perform png image compression using oxipng. It takes the input file name and
/// the output filename as argument and executes the default oxipng compression
/// logic.
///
/// # Arguments
///
/// * `inputfile` - String input png filename
/// * `outputfile` - String output png filename
///
/// # Examples
///
/// ```
/// perform("./website/static/img/demo.png", "./dist/static/demo.png")
/// ```
fn perform(inputfile: String, outputfile: String) -> String {
    let mut options = oxipng::Options::from_preset(5);
    options.timeout = Some(Duration::from_secs(2));
    let infile = oxipng::InFile::Path(PathBuf::from(inputfile));
    let outfile = oxipng::OutFile::Path(Some(PathBuf::from(outputfile)));
    if let Ok(_) = oxipng::optimize(&infile, &outfile, &options) {
        return "done".to_string();
    } else {
        return "error".to_string();
    };
}

/// Compress function to compress `png` files using onxipng.
/// It spawns up some amount of threads, (configurable via PNG_COMPRESS_THREAD env
/// variable), default value is 8. Them it basically chunked the complete array
/// which is sent for processing having the following structure.
///
/// {
///  in: "string",
///  out: "string"
/// }
///
/// These entries will create `CompressTask` object which will be delegated to oxinpng
/// for handling the compression.
///
/// It iterates on the array which is sent from the calle function to this as an argument.
///
/// # Arguments
///
/// * `cx` - Function context created by neon binding
fn compress(mut cx: FunctionContext) -> JsResult<JsNumber> {
    let js_arr_handle: Handle<JsArray> = cx.argument(0)?;
    let mut vec: Vec<Handle<JsValue>> = js_arr_handle.to_vec(&mut cx)?;
    let compress_arr: Vec<CompressTask> = vec
        .iter_mut()
        .map(|val| {
            return create_compress_task(val, &mut cx);
        })
        .collect();

    let arc_compress_arr = Arc::new(compress_arr);
    let mut handles = vec![];
    let num_threads: usize = option_env!("PNG_COMPRESS_THREADS")
        .unwrap_or("8")
        .to_string()
        .parse::<usize>()
        .unwrap();
    for _ in 0..num_threads {
        let data_clone = arc_compress_arr.clone();
        let th = thread::spawn(move || {
            for item in data_clone.chunks(num_threads) {
                let input = item[0].input.to_string();
                let out = item[0].out.to_string();
                perform(input, out);
            }
        });
        handles.push(th);
    }
    for handle in handles {
        handle.join().unwrap();
    }
    Ok(cx.number(vec.len() as f64))
}

fn create_compress_task(val: &mut Handle<JsValue>, cx: &mut CallContext<JsObject>) -> CompressTask {
    let js_object = val
        .downcast::<JsObject, FunctionContext>(cx)
        .or_throw(cx)
        .unwrap();
    let infilename = js_object
        .get(cx, "in")
        .unwrap()
        .downcast::<JsString, FunctionContext>(cx)
        .or_throw(cx)
        .unwrap()
        .value(cx);
    let outfilename = js_object
        .get(cx, "out")
        .unwrap()
        .downcast::<JsString, FunctionContext>(cx)
        .or_throw(cx)
        .unwrap()
        .value(cx);
    return CompressTask {
        input: infilename,
        out: outfilename,
    };
}

register_module!(mut m, { m.export_function("compress", compress) });
