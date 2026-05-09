use oneway::checker;
use oneway::codegen;
use oneway::error::OnewayError;
use oneway::lexer::Scanner;
use oneway::parser::Parser;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: oneway <file.ow> [--tokens|--ast|--check|--emit-rust|--compile]");
        process::exit(1);
    }

    let mut file_path: Option<&str> = None;
    let mut mode = "default";

    for arg in &args[1..] {
        match arg.as_str() {
            "--tokens" => mode = "tokens",
            "--ast" => mode = "ast",
            "--check" => mode = "check",
            "--emit-rust" => mode = "emit-rust",
            "--compile" => mode = "compile",
            s if s.starts_with('-') => {
                eprintln!("Unknown flag: {}", s);
                eprintln!("Usage: oneway <file.ow> [--tokens|--ast|--check|--emit-rust|--compile]");
                process::exit(1);
            }
            s => {
                if file_path.is_some() {
                    eprintln!("Error: multiple input files are not supported");
                    process::exit(1);
                }
                file_path = Some(s);
            }
        }
    }

    let file_path = match file_path {
        Some(p) => p,
        None => {
            eprintln!("Error: no input file provided");
            eprintln!("Usage: oneway <file.ow> [--tokens|--ast|--check|--emit-rust|--compile]");
            process::exit(1);
        }
    };

    // Read source
    let source = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error: could not read '{}': {}", file_path, err);
            process::exit(1);
        }
    };

    // Step 1: Lex
    let mut scanner = Scanner::new(&source);
    let tokens = match scanner.scan_tokens() {
        Ok(tokens) => tokens,
        Err(err) => {
            print_error(file_path, &err);
            process::exit(1);
        }
    };

    if mode == "tokens" {
        for token in &tokens {
            println!(
                "{:>4}:{:<4} {:<20} {:?}",
                token.span.line, token.span.column, token.kind, token.lexeme
            );
        }
        return;
    }

    // Step 2: Parse
    let mut parser = Parser::new(tokens);
    let module = match parser.parse() {
        Ok(module) => module,
        Err(err) => {
            print_error(file_path, &err);
            process::exit(1);
        }
    };

    if mode == "ast" {
        println!("{:#?}", module);
        return;
    }

    // Step 3: Check
    let errors = checker::check(&module);
    if !errors.is_empty() {
        for err in &errors {
            print_error(file_path, err);
        }
        if mode == "check" {
            eprintln!("\n{} error(s) found.", errors.len());
            process::exit(1);
        }
        // For other modes, still show errors but continue
        eprintln!("\nWarning: {} sort-order error(s) found.", errors.len());
    } else if mode == "check" {
        println!("All checks passed.");
        return;
    }

    if mode == "check" {
        return;
    }

    // Step 4: Generate Rust
    let rust_code = codegen::generate(&module);

    if mode == "emit-rust" || mode == "default" {
        println!("{}", rust_code);
        return;
    }

    if mode == "compile" {
        // Write to temp file, invoke rustc
        let out_path = file_path.replace(".ow", "");
        let rs_path = format!("{}.rs", out_path);
        if let Err(err) = fs::write(&rs_path, &rust_code) {
            eprintln!("Error writing {}: {}", rs_path, err);
            process::exit(1);
        }

        let status = std::process::Command::new("rustc")
            .arg(&rs_path)
            .arg("-o")
            .arg(&out_path)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!("Compiled to: {}", out_path);
                // Clean up .rs file
                let _ = fs::remove_file(&rs_path);
            }
            Ok(s) => {
                eprintln!("rustc failed with: {}", s);
                process::exit(1);
            }
            Err(err) => {
                eprintln!("Failed to run rustc: {}", err);
                process::exit(1);
            }
        }
    }
}

fn print_error(file_path: &str, err: &OnewayError) {
    let span = err.span();
    eprintln!(
        "error[{}:{}:{}]: {}",
        file_path,
        span.line,
        span.column,
        err.message()
    );
}
