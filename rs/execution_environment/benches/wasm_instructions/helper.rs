//! Helper Functions

use crate::common::Wasm64;
use execution_environment_bench::{
    common,
    wat_builder::{
        Block, CONFIRMATION_LOOP_ITERATIONS, CONFIRMATION_REPEAT_TIMES, DEFAULT_LOOP_ITERATIONS,
        DEFAULT_REPEAT_TIMES,
    },
};

/// Run all the benchmark or just the first in a group.
const RUN_ALL_BENCHMARKS: bool = true;

/// Returns either the first or all the benchmarks.
pub fn first_or_all<'a>(all: &'a [&'a str]) -> &'a [&'a str] {
    if RUN_ALL_BENCHMARKS {
        all
    } else {
        &all[..1]
    }
}

/// Creates a benchmark with its confirmation for the specified `code` snippet.
///
/// The confirmation benchmark is to make sure there is no compiler optimization
/// for the repeated lines of code.
pub fn benchmark_with_confirmation(name: &str, code: &str) -> Vec<common::Benchmark> {
    let i = DEFAULT_LOOP_ITERATIONS;
    let r = DEFAULT_REPEAT_TIMES;
    let c = CONFIRMATION_REPEAT_TIMES;

    vec![
        benchmark(&format!("wasm32/{name}"), i, r, code),
        benchmark(&format!("wasm32/{name}/confirmation"), i, c, code),
        benchmark(&format!("wasm64/{name}"), i, r, code),
        benchmark(&format!("wasm64/{name}/confirmation"), i, c, code),
    ]
}

/// Creates a benchmark with its confirmation for the specified `code` snippet.
///
/// The confirmation benchmark is to make sure there is no compiler optimization
/// for the loop.
pub fn benchmark_with_loop_confirmation(name: &str, code: &str) -> Vec<common::Benchmark> {
    let i = DEFAULT_LOOP_ITERATIONS;
    let c = CONFIRMATION_LOOP_ITERATIONS;
    let r = DEFAULT_REPEAT_TIMES;
    vec![
        benchmark(&format!("wasm32/{name}"), i, r, code),
        benchmark(&format!("wasm32/{name}/confirmation"), c, r, code),
        benchmark(&format!("wasm64/{name}"), i, r, code),
        benchmark(&format!("wasm64/{name}/confirmation"), c, r, code),
    ]
}

/// Creates a benchmark with a code block repeated specified number of times in a loop.
pub fn benchmark(name: &str, i: usize, r: usize, repeat_code: &str) -> common::Benchmark {
    let wasm64_enabled = name.starts_with("wasm64");
    let repeat_code = if wasm64_enabled {
        repeat_code.replace("memtype", "i64")
    } else {
        repeat_code.replace("memtype", "i32")
    };
    let wasm64_enabled = if wasm64_enabled {
        Wasm64::Enabled
    } else {
        Wasm64::Disabled
    };
    common::Benchmark(
        name.into(),
        Block::default()
            .repeat_n(r, &repeat_code)
            .loop_n(i)
            .define_variables_and_functions(&repeat_code)
            .into_update_func()
            .into_test_module_wat(wasm64_enabled),
        (i * r) as u64,
    )
}
