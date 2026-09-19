use clap::{Args, Parser, Subcommand, ValueEnum};
use owo_colors::OwoColorize;
use rrc_core::{
    decode, encode, render_ascii, render_jpeg, render_png, render_svg, version_info, Color,
    DecodeOptions, EccLevel, EncodeOptions, RrcStyle, SectorStyle,
};
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "rrc",
    author = "Ryan Shelby <MdSagorMunshi>",
    version = "0.1.0",
    about = "Radial Response Code (RRC) — Reference CLI Encoder & Decoder",
    long_about = "RRC is an original polar/radial 2D optical code symbology featuring 360-degree rotation invariance, concentric data rings, and Reed-Solomon error correction."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Encode data into an RRC symbol
    Encode(EncodeArgs),
    /// Decode an RRC symbol from an image
    Decode(DecodeArgs),
    /// Display capacity and geometry specifications for RRC versions
    Info(InfoArgs),
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliEccLevel {
    L,
    M,
    Q,
    H,
}

impl From<CliEccLevel> for EccLevel {
    fn from(c: CliEccLevel) -> Self {
        match c {
            CliEccLevel::L => EccLevel::L,
            CliEccLevel::M => EccLevel::M,
            CliEccLevel::Q => EccLevel::Q,
            CliEccLevel::H => EccLevel::H,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliSectorStyle {
    Sharp,
    Rounded,
    Pill,
    InnerRounded,
}

impl From<CliSectorStyle> for SectorStyle {
    fn from(s: CliSectorStyle) -> Self {
        match s {
            CliSectorStyle::Sharp => SectorStyle::Sharp,
            CliSectorStyle::Rounded => SectorStyle::Rounded,
            CliSectorStyle::Pill => SectorStyle::Pill,
            CliSectorStyle::InnerRounded => SectorStyle::InnerRounded,
        }
    }
}

#[derive(Args)]
struct EncodeArgs {
    /// Data payload to encode into the RRC symbol
    data: String,

    /// Output file path (.svg, .png, .jpg / .jpeg)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Target symbol version (1..=40). If omitted, smallest fitting version is selected.
    #[arg(short, long)]
    version: Option<u8>,

    /// Error correction level: L (~7%), M (~15%), Q (~25%), H (~30%)
    #[arg(short, long, value_enum, default_value_t = CliEccLevel::M)]
    ecc: CliEccLevel,

    /// Sector visual style: sharp, rounded, pill, inner-rounded
    #[arg(long, value_enum, default_value_t = CliSectorStyle::Sharp)]
    style: CliSectorStyle,

    /// Output render scale in pixels per unit (default: 10)
    #[arg(long, default_value_t = 10)]
    scale: u32,

    /// Foreground/dark color in hex (e.g. '#00FF94' or '#000000')
    #[arg(long, default_value = "#000000")]
    dark_color: String,

    /// Background/light color in hex (e.g. '#0A0A0A' or '#FFFFFF')
    #[arg(long, default_value = "#FFFFFF")]
    light_color: String,

    /// Bullseye center color in hex (defaults to dark color)
    #[arg(long)]
    bullseye_color: Option<String>,

    /// Render symbol as Unicode Braille art in terminal
    #[arg(long)]
    braille: bool,

    /// Render symbol as standard ASCII art in terminal
    #[arg(long)]
    ascii: bool,
}

#[derive(Args)]
struct DecodeArgs {
    /// Path to the symbol image file (.png, .jpg, .jpeg, etc.)
    image: PathBuf,

    /// Expected version number if known
    #[arg(long)]
    version: Option<u8>,

    /// Enable verbose diagnostic scanning output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Args)]
struct InfoArgs {
    /// Version number (1..=40) to inspect. If omitted, full summary table is displayed.
    version: Option<u8>,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Encode(args) => handle_encode(args),
        Commands::Decode(args) => handle_decode(args),
        Commands::Info(args) => handle_info(args),
    }
}

fn handle_encode(args: EncodeArgs) {
    let dark = Color::from_hex(&args.dark_color).unwrap_or_else(|e| {
        eprintln!("{}: {}", "Error".red().bold(), e);
        process::exit(1);
    });
    let light = Color::from_hex(&args.light_color).unwrap_or_else(|e| {
        eprintln!("{}: {}", "Error".red().bold(), e);
        process::exit(1);
    });
    let bullseye = args
        .bullseye_color
        .map(|c| {
            Color::from_hex(&c).unwrap_or_else(|e| {
                eprintln!("{}: {}", "Error".red().bold(), e);
                process::exit(1);
            })
        });

    let mut style = RrcStyle::default();
    style.dark_color = dark;
    style.light_color = light;
    style.bullseye_dark = bullseye;
    style.sector_style = args.style.into();
    style.render_scale = args.scale;

    let mut opts = EncodeOptions::default();
    opts.version = args.version;
    opts.ecc_level = args.ecc.into();
    opts.style = style.clone();

    let symbol = match encode(args.data.as_bytes(), &opts) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {}", "Encoding failed".red().bold(), e);
            process::exit(1);
        }
    };

    println!(
        "{} RRC Version {} ({} data rings, ECC {:?}, Mask #{})",
        "✓ Generated".green().bold(),
        symbol.version.bold(),
        symbol.matrix.len().cyan(),
        opts.ecc_level.yellow(),
        symbol.mask_index.cyan()
    );

    if let Some(ref out_path) = args.output {
        let ext = out_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "svg" => {
                let svg = render_svg(symbol.version, &symbol.matrix, &symbol.style);
                if let Err(e) = fs::write(out_path, svg) {
                    eprintln!("{}: Failed to write SVG file: {e}", "Error".red().bold());
                    process::exit(1);
                }
                println!("{} Saved SVG to {}", "➜".green().bold(), out_path.display().cyan());
            }
            "png" => {
                let png_bytes = match render_png(symbol.version, &symbol.matrix, &symbol.style) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("{}: Failed to render PNG: {e}", "Error".red().bold());
                        process::exit(1);
                    }
                };
                if let Err(e) = fs::write(out_path, png_bytes) {
                    eprintln!("{}: Failed to write PNG file: {e}", "Error".red().bold());
                    process::exit(1);
                }
                println!("{} Saved PNG to {}", "➜".green().bold(), out_path.display().cyan());
            }
            "jpg" | "jpeg" => {
                let jpg_bytes = match render_jpeg(symbol.version, &symbol.matrix, &symbol.style, None) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("{}: Failed to render JPEG: {e}", "Error".red().bold());
                        process::exit(1);
                    }
                };
                if let Err(e) = fs::write(out_path, jpg_bytes) {
                    eprintln!("{}: Failed to write JPEG file: {e}", "Error".red().bold());
                    process::exit(1);
                }
                println!("{} Saved JPEG to {}", "➜".green().bold(), out_path.display().cyan());
            }
            other => {
                eprintln!(
                    "{}: Unsupported output extension '.{other}'. Use .svg, .png, or .jpg",
                    "Error".red().bold()
                );
                process::exit(1);
            }
        }
    }

    // Terminal text rendering
    if args.output.is_none() || args.braille || args.ascii {
        let mut term_style = style;
        term_style.ascii_use_braille = !args.ascii; // Default to braille if not ascii flag
        println!("\n{}", render_ascii(symbol.version, &symbol.matrix, &term_style));
    }
}

fn handle_decode(args: DecodeArgs) {
    if !args.image.exists() {
        eprintln!("{}: Image file '{}' not found", "Error".red().bold(), args.image.display());
        process::exit(1);
    }

    let img = match image::open(&args.image) {
        Ok(im) => im.to_rgba8(),
        Err(e) => {
            eprintln!("{}: Failed to open image: {e}", "Error".red().bold());
            process::exit(1);
        }
    };

    let (width, height) = img.dimensions();
    let raw_rgba = img.into_raw();

    let dec_opts = DecodeOptions {
        verbose: args.verbose,
        expected_version: args.version,
    };

    println!(
        "{} Scanning {} ({}x{} pixels)...",
        "⚙".cyan().bold(),
        args.image.display().bold(),
        width.cyan(),
        height.cyan()
    );

    match decode(&raw_rgba, width, height, &dec_opts) {
        Ok(res) => {
            println!("\n{}", "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓".green().bold());
            println!(
                "{}  {}",
                "┃".green().bold(),
                "RADIAL RESPONSE CODE (RRC) — DECODE SUCCESS".green().bold()
            );
            println!("{}", "┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫".green().bold());
            println!("{}  Version:       {}", "┃".green().bold(), res.version.bold());
            println!("{}  ECC Level:     {:?}", "┃".green().bold(), res.ecc_level.bold());
            println!("{}  Encoding Mode: {:?}", "┃".green().bold(), res.mode.bold());
            println!("{}  Byte Length:   {} bytes", "┃".green().bold(), res.payload.len().bold());
            println!("{}", "┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫".green().bold());
            if let Some(text) = &res.text {
                println!("{}  {}", "┃".green().bold(), "DECODED TEXT:".bright_yellow().bold());
                for line in text.lines() {
                    println!("{}    {}", "┃".green().bold(), line.white().bold());
                }
            } else {
                println!("{}  {}", "┃".green().bold(), "RAW BINARY HEX:".bright_yellow().bold());
                let hex_str: String = res.payload.iter().map(|b| format!("{b:02X} ")).collect();
                println!("{}    {}", "┃".green().bold(), hex_str.white());
            }
            println!("{}\n", "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛".green().bold());
        }
        Err(e) => {
            eprintln!("\n{}: {}", "Decode failed".red().bold(), e);
            process::exit(1);
        }
    }
}

fn handle_info(args: InfoArgs) {
    if let Some(v) = args.version {
        if !(1..=40).contains(&v) {
            eprintln!("{}: Version must be between 1 and 40", "Error".red().bold());
            process::exit(1);
        }
        let info = version_info(v).unwrap();
        println!("\n{}", format!("=== RRC Version {v} Specifications ===").green().bold());
        println!("  Concentric Data Rings:  {}", info.ring_count.cyan());
        println!("  Base Ring Sectors:      {}", info.base_sectors.cyan());
        println!("  Total Data Bits:        {}", info.total_data_bits.cyan());
        println!("  Total Codewords:        {}", info.total_codewords.cyan());
        println!("  ECC Codewords (L / M / Q / H):  {} / {} / {} / {}",
            info.ecc_codewords_l.yellow(),
            info.ecc_codewords_m.yellow(),
            info.ecc_codewords_q.yellow(),
            info.ecc_codewords_h.yellow()
        );
        println!("  Data Capacity (Bytes):");
        println!("    Level L (~7%):   {} bytes", (info.total_codewords - info.ecc_codewords_l).green());
        println!("    Level M (~15%):  {} bytes", info.capacity_bytes_m.green().bold());
        println!("    Level Q (~25%):  {} bytes", (info.total_codewords - info.ecc_codewords_q).green());
        println!("    Level H (~30%):  {} bytes", (info.total_codewords - info.ecc_codewords_h).green());
        println!("  Alphanumeric Capacity (M): {} chars", info.capacity_alpha_m.cyan());
        println!("  Numeric Capacity (M):      {} digits\n", info.capacity_numeric_m.cyan());
    } else {
        println!("\n{}", "=== RRC Version Capacity Table (V1 - V40) ===".green().bold());
        println!(
            "{:<4} {:<6} {:<10} {:<10} {:<12} {:<12} {:<10}",
            "Ver", "Rings", "Codewords", "Cap(L)", "Cap(M)", "Cap(Q)", "Cap(H)"
        );
        println!("{}", "─".repeat(68).dimmed());

        for v in 1..=40 {
            let info = version_info(v).unwrap();
            let cap_l = info.total_codewords - info.ecc_codewords_l;
            let cap_q = info.total_codewords - info.ecc_codewords_q;
            let cap_h = info.total_codewords - info.ecc_codewords_h;

            println!(
                "{:<4} {:<6} {:<10} {:<12} {:<12} {:<12} {:<10}",
                format!("V{v}").bold(),
                info.ring_count,
                info.total_codewords,
                format!("{cap_l} B").green(),
                format!("{} B", info.capacity_bytes_m).green().bold(),
                format!("{cap_q} B").yellow(),
                format!("{cap_h} B").red()
            );
        }
        println!();
    }
}
