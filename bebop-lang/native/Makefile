# Bebop native compiler — Phase 1 (C bootstrap)
# Native, not Rust. Zero external dependencies (libc only). Self-hosts later.
CC      ?= cc
CFLAGS  ?= -O2 -std=c11 -Wall -Wextra -Wpedantic
BUILD   ?= build
BIN     := $(BUILD)/bebopc

SRC := src/main.c src/glyph.c src/lexer.c src/parser.c

all: $(BIN)

$(BIN): $(SRC) src/glyph.h src/lexer.h src/parser.h
	@mkdir -p $(BUILD)
	$(CC) $(CFLAGS) -o $@ $(SRC)

test: $(BIN)
	$(BIN) version
	$(BIN) glyphs
	$(BIN) glyph fn
	$(BIN) glyph struct
	@echo "--- tokenize sample ---"
	$(BIN) tokens ../samples/hello.bp
	@echo "--- parse sample ---"
	$(BIN) parse ../samples/hello.bp

clean:
	rm -rf $(BUILD)

.PHONY: all test clean
