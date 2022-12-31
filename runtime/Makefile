WITBINDGEN ?= wit-bindgen
CARGO ?= cargo

#generate-api:
#	$(WITBINDGEN) host wasmtime-rust --rustfmt spec/state.wit --out-dir vm/src
#	$(WITBINDGEN) host wasmtime-rust --rustfmt spec/app.wit --out-dir vm/src
#	$(WITBINDGEN) host wasmtime-rust --rustfmt spec/runtime.wit --out-dir vm/src
#	$(WITBINDGEN) host wasmtime-rust --rustfmt spec/storage.wit --out-dir vm/src
generate-sdk:
	rm -rf "generated"
	mkdir "generated"
	$(WITBINDGEN) guest rust --rustfmt spec/state.wit --out-dir generated
	$(WITBINDGEN) guest rust --rustfmt --macro-export spec/app.wit --out-dir generated
	$(WITBINDGEN) guest rust --rustfmt spec/runtime.wit --out-dir generated
	$(WITBINDGEN) guest rust --rustfmt spec/storage.wit --out-dir generated

