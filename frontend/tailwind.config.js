/**
 * BSL Gradual Type System - Tailwind CSS Configuration
 *
 * CSS Variables → Tailwind Mapping Table
 * ==========================================
 *
 * COLORS:
 * -------
 * Primitives:
 * --color-white → bg-white / text-white
 * --color-black → bg-black / text-black
 * --color-cream-50 → bg-bsl-cream-50 (252, 252, 249)
 * --color-cream-100 → bg-bsl-cream-100 (255, 255, 253)
 * --color-gray-200 → bg-bsl-gray-200 (245, 245, 245)
 * --color-gray-300 → bg-bsl-gray-300 (167, 169, 169)
 * --color-gray-400 → bg-bsl-gray-400 (119, 124, 124)
 * --color-slate-500 → bg-bsl-slate-500 (98, 108, 113)
 * --color-brown-600 → bg-bsl-brown-600 (94, 82, 64)
 * --color-charcoal-700 → bg-bsl-charcoal-700 (31, 33, 33)
 * --color-charcoal-800 → bg-bsl-charcoal-800 (38, 40, 40)
 * --color-slate-900 → bg-bsl-slate-900 (19, 52, 59)
 * --color-teal-300 → bg-bsl-teal-300 (50, 184, 198)
 * --color-teal-400 → bg-bsl-teal-400 (45, 166, 178)
 * --color-teal-500 → bg-bsl-teal-500 (33, 128, 141)
 * --color-teal-600 → bg-bsl-teal-600 (29, 116, 128)
 * --color-teal-700 → bg-bsl-teal-700 (26, 104, 115)
 * --color-teal-800 → bg-bsl-teal-800 (41, 150, 161)
 * --color-red-400 → bg-bsl-red-400 (255, 84, 89)
 * --color-red-500 → bg-bsl-red-500 (192, 21, 47)
 * --color-orange-400 → bg-bsl-orange-400 (230, 129, 97)
 * --color-orange-500 → bg-bsl-orange-500 (168, 75, 47)
 *
 * Semantic (Light Mode):
 * --color-background → bg-bsl-cream-50 (use semantic: bg-bsl-bg)
 * --color-surface → bg-bsl-cream-100 (use semantic: bg-bsl-surface)
 * --color-text → text-bsl-slate-900 (use semantic: text-bsl-text)
 * --color-text-secondary → text-bsl-slate-500/70 (use semantic: text-bsl-text-secondary)
 * --color-primary → bg-bsl-teal-500 (use semantic: bg-bsl-primary)
 * --color-primary-hover → hover:bg-bsl-teal-600
 * --color-primary-active → active:bg-bsl-teal-700
 * --color-border → border-bsl-brown-600/20 (use semantic: border-bsl-border)
 * --color-error → text-bsl-red-500
 * --color-success → text-bsl-teal-500
 * --color-warning → text-bsl-orange-500
 * --color-info → text-bsl-slate-500
 *
 * TYPOGRAPHY:
 * -----------
 * --font-family-base → font-sans (FKGroteskNeue, Geist, Inter, system)
 * --font-family-mono → font-mono (Berkeley Mono, ui-monospace, system)
 * --font-size-xs → text-xs (11px)
 * --font-size-sm → text-sm (12px)
 * --font-size-base → text-base (14px)
 * --font-size-md → text-md (14px - alias of base)
 * --font-size-lg → text-lg (16px)
 * --font-size-xl → text-xl (18px)
 * --font-size-2xl → text-2xl (20px)
 * --font-size-3xl → text-3xl (24px)
 * --font-size-4xl → text-4xl (30px)
 * --font-weight-normal → font-normal (400)
 * --font-weight-medium → font-medium (500)
 * --font-weight-semibold → font-semibold (550)
 * --font-weight-bold → font-bold (600)
 * --line-height-tight → leading-tight (1.2)
 * --line-height-normal → leading-normal (1.5)
 * --letter-spacing-tight → tracking-tight (-0.01em)
 *
 * SPACING:
 * --------
 * --space-0 → p-0 / m-0 / gap-0 (0)
 * --space-1 → p-0.5 / m-0.5 (1px)
 * --space-2 → p-0.5 / m-0.5 (2px)
 * --space-4 → p-1 / m-1 (4px)
 * --space-6 → p-1.5 / m-1.5 (6px)
 * --space-8 → p-2 / m-2 (8px)
 * --space-10 → p-2.5 / m-2.5 (10px)
 * --space-12 → p-3 / m-3 (12px)
 * --space-16 → p-4 / m-4 (16px)
 * --space-20 → p-5 / m-5 (20px)
 * --space-24 → p-6 / m-6 (24px)
 * --space-32 → p-8 / m-8 (32px)
 *
 * BORDER RADIUS:
 * --------------
 * --radius-sm → rounded-sm (6px)
 * --radius-base → rounded (8px)
 * --radius-md → rounded-md (10px)
 * --radius-lg → rounded-lg (12px)
 * --radius-full → rounded-full (9999px)
 *
 * SHADOWS:
 * --------
 * --shadow-xs → shadow-xs
 * --shadow-sm → shadow-sm
 * --shadow-md → shadow-md
 * --shadow-lg → shadow-lg
 * --shadow-inset-sm → shadow-inset-sm (custom)
 *
 * ANIMATION:
 * ----------
 * --duration-fast → duration-fast (150ms)
 * --duration-normal → duration-normal (250ms)
 * --ease-standard → ease-smooth (cubic-bezier(0.16, 1, 0.3, 1))
 *
 * LAYOUT:
 * -------
 * --container-sm → max-w-sm (640px)
 * --container-md → max-w-md (768px)
 * --container-lg → max-w-lg (1024px)
 * --container-xl → max-w-xl (1280px)
 */

/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./src/**/*.rs",
    "./index.html",
    "./type_details.html",
    "./quick_actions.html",
  ],
  darkMode: 'media', // Respects OS dark mode preference
  theme: {
    extend: {
      colors: {
        // VSCode CSS Variables (for webview integration)
        // These map to VSCode's built-in CSS variables
        vscode: {
          bg: 'var(--vscode-editor-background)',
          fg: 'var(--vscode-editor-foreground)',
          'input-bg': 'var(--vscode-input-background)',
          'input-fg': 'var(--vscode-input-foreground)',
          'button-bg': 'var(--vscode-button-background)',
          'button-fg': 'var(--vscode-button-foreground)',
          'button-hover': 'var(--vscode-button-hoverBackground)',
          'list-hover': 'var(--vscode-list-hoverBackground)',
          'border': 'var(--vscode-panel-border)',
          'link': 'var(--vscode-textLink-foreground)',
          'error': 'var(--vscode-errorForeground)',
          'warning': 'var(--vscode-editorWarning-foreground)',
          'success': 'var(--vscode-terminal-ansiGreen)',
        },
        // BSL custom color palette
        bsl: {
          // Primitives
          cream: {
            50: 'rgb(252, 252, 249)',
            100: 'rgb(255, 255, 253)',
          },
          gray: {
            200: 'rgb(245, 245, 245)',
            300: 'rgb(167, 169, 169)',
            400: 'rgb(119, 124, 124)',
          },
          slate: {
            500: 'rgb(98, 108, 113)',
            900: 'rgb(19, 52, 59)',
          },
          brown: {
            600: 'rgb(94, 82, 64)',
          },
          charcoal: {
            700: 'rgb(31, 33, 33)',
            800: 'rgb(38, 40, 40)',
          },
          teal: {
            300: 'rgb(50, 184, 198)',
            400: 'rgb(45, 166, 178)',
            500: 'rgb(33, 128, 141)',
            600: 'rgb(29, 116, 128)',
            700: 'rgb(26, 104, 115)',
            800: 'rgb(41, 150, 161)',
          },
          red: {
            400: 'rgb(255, 84, 89)',
            500: 'rgb(192, 21, 47)',
          },
          orange: {
            400: 'rgb(230, 129, 97)',
            500: 'rgb(168, 75, 47)',
          },

          // Colorful backgrounds (Light Mode)
          bg: {
            1: 'rgba(59, 130, 246, 0.08)',   // Light blue
            2: 'rgba(245, 158, 11, 0.08)',   // Light yellow
            3: 'rgba(34, 197, 94, 0.08)',    // Light green
            4: 'rgba(239, 68, 68, 0.08)',    // Light red
            5: 'rgba(147, 51, 234, 0.08)',   // Light purple
            6: 'rgba(249, 115, 22, 0.08)',   // Light orange
            7: 'rgba(236, 72, 153, 0.08)',   // Light pink
            8: 'rgba(6, 182, 212, 0.08)',    // Light cyan
          },
        },
      },
      fontFamily: {
        sans: [
          'FKGroteskNeue',
          'Geist',
          'Inter',
          '-apple-system',
          'BlinkMacSystemFont',
          'Segoe UI',
          'Roboto',
          'sans-serif',
        ],
        mono: [
          'Berkeley Mono',
          'ui-monospace',
          'SFMono-Regular',
          'Menlo',
          'Monaco',
          'Consolas',
          'monospace',
        ],
      },
      fontSize: {
        xs: '11px',
        sm: '12px',
        base: '14px',
        md: '14px', // Alias of base
        lg: '16px',
        xl: '18px',
        '2xl': '20px',
        '3xl': '24px',
        '4xl': '30px',
      },
      fontWeight: {
        normal: '400',
        medium: '500',
        semibold: '550', // Custom BSL weight
        bold: '600',
      },
      lineHeight: {
        tight: '1.2',
        normal: '1.5',
      },
      letterSpacing: {
        tight: '-0.01em',
      },
      spacing: {
        // Exact BSL spacing values (px → rem/px)
        0: '0',
        0.5: '1px',  // --space-1, --space-2
        1: '4px',    // --space-4
        1.5: '6px',  // --space-6
        2: '8px',    // --space-8
        2.5: '10px', // --space-10
        3: '12px',   // --space-12
        4: '16px',   // --space-16
        5: '20px',   // --space-20
        6: '24px',   // --space-24
        8: '32px',   // --space-32
      },
      borderRadius: {
        sm: '6px',
        DEFAULT: '8px',
        md: '10px',
        lg: '12px',
        full: '9999px',
      },
      boxShadow: {
        xs: '0 1px 2px rgba(0, 0, 0, 0.02)',
        sm: '0 1px 3px rgba(0, 0, 0, 0.04), 0 1px 2px rgba(0, 0, 0, 0.02)',
        md: '0 4px 6px -1px rgba(0, 0, 0, 0.04), 0 2px 4px -1px rgba(0, 0, 0, 0.02)',
        lg: '0 10px 15px -3px rgba(0, 0, 0, 0.04), 0 4px 6px -2px rgba(0, 0, 0, 0.02)',
        'inset-sm': 'inset 0 1px 0 rgba(255, 255, 255, 0.15), inset 0 -1px 0 rgba(0, 0, 0, 0.03)',
      },
      transitionDuration: {
        fast: '150ms',
        normal: '250ms',
      },
      transitionTimingFunction: {
        smooth: 'cubic-bezier(0.16, 1, 0.3, 1)', // --ease-standard
      },
      maxWidth: {
        sm: '640px',
        md: '768px',
        lg: '1024px',
        xl: '1280px',
      },
      animation: {
        'fade-in': 'fadeIn 0.2s ease-out',
        'slide-in': 'slideIn 0.3s ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideIn: {
          '0%': { transform: 'translateY(20px)', opacity: '0' },
          '100%': { transform: 'translateY(0)', opacity: '1' },
        },
      },
    },
  },
  plugins: [],
}
