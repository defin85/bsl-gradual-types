//! CSS styles generation for semantic HTML visualization
//!
//! Contains color schemes and inline CSS generation for HTML output.

/// Color scheme for different themes
pub(crate) struct ColorScheme {
    pub bg_primary: &'static str,
    pub bg_secondary: &'static str,
    pub text_primary: &'static str,
    pub text_secondary: &'static str,
    pub border_color: &'static str,
    pub accent_procedure: &'static str,
    pub accent_function: &'static str,
    pub accent_variable: &'static str,
    pub accent_assignment: &'static str,
    pub code_bg: &'static str,
}

impl ColorScheme {
    pub fn dark() -> Self {
        Self {
            bg_primary: "#111827",
            bg_secondary: "#1f2937",
            text_primary: "#f3f4f6",
            text_secondary: "#d1d5db",
            border_color: "#374151",
            accent_procedure: "#60a5fa",
            accent_function: "#34d399",
            accent_variable: "#fbbf24",
            accent_assignment: "#c084fc",
            code_bg: "#0f172a",
        }
    }

    pub fn light() -> Self {
        Self {
            bg_primary: "#ffffff",
            bg_secondary: "#f9fafb",
            text_primary: "#111827",
            text_secondary: "#6b7280",
            border_color: "#e5e7eb",
            accent_procedure: "#2563eb",
            accent_function: "#059669",
            accent_variable: "#d97706",
            accent_assignment: "#7c3aed",
            code_bg: "#f3f4f6",
        }
    }
}

/// Generates inline CSS styles
pub(crate) fn generate_inline_css(colors: &ColorScheme) -> String {
    format!(
        r#"        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            background-color: {bg_primary};
            color: {text_primary};
            font-family: 'Segoe UI', 'Consolas', 'Monaco', 'Courier New', monospace;
            line-height: 1.6;
            font-size: 14px;
        }}

        .container {{
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }}

        header {{
            background-color: {bg_secondary};
            border-bottom: 2px solid {border_color};
            padding: 30px;
            border-radius: 8px;
            margin-bottom: 30px;
        }}

        h1 {{
            font-size: 28px;
            margin-bottom: 10px;
            font-weight: 700;
        }}

        h2 {{
            font-size: 22px;
            margin-bottom: 20px;
            padding-bottom: 10px;
            border-bottom: 2px solid {border_color};
            font-weight: 600;
        }}

        h3 {{
            font-size: 18px;
            margin-bottom: 10px;
            font-weight: 600;
        }}

        .subtitle {{
            color: {text_secondary};
            font-size: 14px;
            margin-bottom: 15px;
        }}

        .meta-info {{
            display: flex;
            gap: 20px;
            margin-top: 15px;
            flex-wrap: wrap;
        }}

        .meta-info span {{
            display: flex;
            align-items: center;
            gap: 8px;
            padding: 8px 12px;
            background-color: {bg_primary};
            border-radius: 4px;
            font-size: 13px;
        }}

        main {{
            display: flex;
            flex-direction: column;
            gap: 30px;
        }}

        .section {{
            background-color: {bg_secondary};
            border: 1px solid {border_color};
            padding: 20px;
            border-radius: 8px;
        }}

        .nodes-container,
        .symbol-table-container {{
            display: grid;
            grid-auto-flow: row;
            gap: 12px;
        }}

        .node {{
            background-color: {bg_primary};
            border-left: 4px solid;
            padding: 15px;
            border-radius: 4px;
            margin-bottom: 8px;
        }}

        .node-procedure {{
            border-left-color: {accent_procedure};
        }}

        .node-function {{
            border-left-color: {accent_function};
        }}

        .node-variable {{
            border-left-color: {accent_variable};
        }}

        .node-assignment {{
            border-left-color: {accent_assignment};
        }}

        .node-header {{
            display: flex;
            align-items: center;
            gap: 10px;
            margin-bottom: 8px;
            font-weight: 600;
        }}

        .node-kind {{
            font-size: 12px;
            padding: 4px 8px;
            border-radius: 3px;
            background-color: {code_bg};
            color: {text_secondary};
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}

        .node-name {{
            font-size: 14px;
            font-weight: 700;
            font-family: 'Consolas', 'Monaco', monospace;
            color: {text_primary};
        }}

        .node-details {{
            margin-top: 10px;
            padding-top: 10px;
            border-top: 1px solid {border_color};
            font-size: 13px;
        }}

        .detail-line {{
            display: flex;
            gap: 15px;
            margin-bottom: 5px;
        }}

        .detail-label {{
            font-weight: 600;
            color: {text_secondary};
            min-width: 100px;
        }}

        .detail-value {{
            color: {text_primary};
            font-family: 'Consolas', 'Monaco', monospace;
        }}

        table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 15px;
        }}

        th {{
            background-color: {bg_primary};
            padding: 12px;
            text-align: left;
            font-weight: 600;
            border-bottom: 2px solid {border_color};
            font-size: 13px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }}

        td {{
            padding: 12px;
            border-bottom: 1px solid {border_color};
        }}

        tr:hover {{
            background-color: {bg_primary};
        }}

        code {{
            background-color: {code_bg};
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Consolas', 'Monaco', monospace;
            font-size: 13px;
            color: {text_secondary};
        }}

        .info-block {{
            background-color: {bg_primary};
            border-left: 4px solid {accent_procedure};
            padding: 20px;
            border-radius: 4px;
        }}

        .info-block ul {{
            margin: 15px 0 0 0;
            padding-left: 25px;
        }}

        .info-block li {{
            margin-bottom: 8px;
        }}

        footer {{
            background-color: {bg_secondary};
            border-top: 2px solid {border_color};
            padding: 20px;
            text-align: center;
            margin-top: 40px;
            border-radius: 8px;
            color: {text_secondary};
            font-size: 13px;
        }}

        /* Responsive design */
        @media (max-width: 768px) {{
            .container {{
                padding: 15px;
            }}

            h1 {{
                font-size: 22px;
            }}

            h2 {{
                font-size: 18px;
            }}

            .meta-info {{
                flex-direction: column;
                gap: 10px;
            }}

            .node {{
                padding: 12px;
            }}

            table {{
                font-size: 12px;
            }}

            th, td {{
                padding: 8px;
            }}
        }}"#,
        bg_primary = colors.bg_primary,
        bg_secondary = colors.bg_secondary,
        text_primary = colors.text_primary,
        text_secondary = colors.text_secondary,
        border_color = colors.border_color,
        accent_procedure = colors.accent_procedure,
        accent_function = colors.accent_function,
        accent_variable = colors.accent_variable,
        accent_assignment = colors.accent_assignment,
        code_bg = colors.code_bg,
    )
}
