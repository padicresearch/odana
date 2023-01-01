WITBINDGEN ?= wit-bindgen

generate:
	rm -rf "generated"
	mkdir "generated"
	$(WITBINDGEN) guest rust --rustfmt --macro-export spec/core.wit --out-dir generated
	$(WITBINDGEN) guest rust --rustfmt spec/app.wit --out-dir generated
	$(WITBINDGEN) guest rust --rustfmt spec/io.wit --out-dir generated