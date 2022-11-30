#WITBINDGEN ?= wit-bindgen
#
#generate-runtime-api:
#	$(WITBINDGEN) host wasmtime-rust --import runtime/spec/state.wit --name state --out-dir runtime/src
#	$(WITBINDGEN) host wasmtime-rust --import runtime/spec/runtime.wit --name runtime --out-dir runtime/src
#	$(WITBINDGEN) host wasmtime-rust --import runtime/spec/storage.wit --name storage --out-dir runtime/src
#	$(WITBINDGEN) host wasmtime-rust --export runtime/spec/executor.wit --name executor --out-dir runtime/src
#generate-runtime-sdk:
#	$(WITBINDGEN) guest rust --import runtime/spec/state.wit --name state --out-dir runtime-sdk/src
#	$(WITBINDGEN) guest rust --import runtime/spec/runtime.wit --name runtime --out-dir runtime-sdk/src
#	$(WITBINDGEN) guest rust --import runtime/spec/storage.wit --name storage --out-dir runtime-sdk/src
#	$(WITBINDGEN) guest rust --export runtime/spec/executor.wit --name executor --out-dir runtime-sdk/src