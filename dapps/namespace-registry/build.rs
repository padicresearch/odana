/*
 * Copyright (c) 2023 Padic Research.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use wasm_builder::WasmBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config
        .extern_path(".odana.types", "::types::prelude")
        .extern_path(".odana.primitive_types", "::primitive_types")
        .format(true);
    let mut extra_build = prost_extra_build::Builder::new();
    extra_build.configure(
        &mut config,
        &[&"proto/name-registery.proto".to_string()],
        &[&"proto".to_string(), &"../../proto".to_string()],
    )?;

    rune_build::configure()
        .out_dir("src")
        .compile_with_config(
            config,
            &[&"proto/name-registery.proto".to_string()],
            &[&"proto".to_string(), &"../../proto".to_string()],
        )
        .unwrap();

    WasmBuilder::new().with_current_project().build();
    Ok(())
}
