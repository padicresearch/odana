WITBINDGEN ?= wit-bindgen
CARGO ?= cargo

generate-api:
	$(WITBINDGEN) host wasmtime-rust --rustfmt --import spec/state.wit --name state --out-dir api/src/generated
	$(WITBINDGEN) host wasmtime-rust --rustfmt --import spec/app.wit --name runtime --out-dir api/src/generated
	$(WITBINDGEN) host wasmtime-rust --rustfmt --import spec/runtime.wit --name runtime --out-dir api/src/generated
	$(WITBINDGEN) host wasmtime-rust --rustfmt --import spec/storage.wit --name storage --out-dir api/src/generated
generate-sdk:
	rm -rf "generated"
	mkdir "generated"
	$(WITBINDGEN) guest rust --rustfmt --import spec/state.wit --name state --out-dir generated
	$(WITBINDGEN) guest rust --rustfmt --macro-export --export spec/app.wit --name app --out-dir generated
	$(WITBINDGEN) guest rust --rustfmt --import spec/runtime.wit --name runtime --out-dir generated
	$(WITBINDGEN) guest rust --rustfmt --import spec/storage.wit --name storage --out-dir generated

