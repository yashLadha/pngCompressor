use neon::prelude::*;
use oxipng;
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
    match oxipng::optimize(&infile, &outfile, &options) {
        Ok(_) => String::from("done"),
        Err(_) => String::from("error"),
    }
}

/// Compress function to compress `png` files using onxipng.
/// It spawns up some amount of threads, (configurable via PNG_COMPRESS_THREADS env
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
    let compress_arr = {
        let arr = vec
            .iter_mut()
            .map(|val| create_compress_task(val, &mut cx))
            .collect::<Vec<CompressTask>>();

        Arc::new(arr)
    };

    let mut handles = vec![];
    // Check for the env variable `PNG_COMPRESS_THREADS` for the value or fallback to the default
    // value of 8. If the passed array size is less than the no of threads set or even the default
    // fallback them use the array length to evenly distribute the load. In case of an invalid
    // value set run it in the single thread.
    let num_threads: usize = option_env!("PNG_COMPRESS_THREADS")
        .unwrap_or("8")
        .to_string()
        .parse::<usize>()
        .unwrap_or(1)
        .max(compress_arr.len());

    let chunk_size = (compress_arr.len() as f64 / num_threads as f64).ceil() as usize;
    for idx in 0..num_threads {
        let data_clone = compress_arr.clone();
        let th = thread::spawn(move || {
            data_clone
                .chunks(chunk_size)
                .nth(idx)
                .unwrap()
                .iter()
                .for_each(|item| {
                    let input = item.input.to_string();
                    let out = item.out.to_string();
                    perform(input, out);
                })
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
    CompressTask {
        input: infilename,
        out: outfilename,
    }
}

register_module!(mut m, { m.export_function("compress", compress) });
