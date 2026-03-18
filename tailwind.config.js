/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{svelte,js,ts}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        surface: {
          50: "#f8fafc",
          100: "#1e1e2e",
          200: "#181825",
          300: "#11111b",
          400: "#0a0a14",
        },
        accent: {
          DEFAULT: "#89b4fa",
          hover: "#74c7ec",
          dim: "#45475a",
        },
      },
      animation: {
        breathe: "breathe 3s ease-in-out infinite",
        "slide-in": "slideIn 0.3s ease-out forwards",
        "fade-in": "fadeIn 0.2s ease-out forwards",
        "ring-spin": "ringSpin 1.2s linear infinite",
      },
      keyframes: {
        breathe: {
          "0%, 100%": { transform: "scale(1)", opacity: "0.85" },
          "50%": { transform: "scale(1.08)", opacity: "1" },
        },
        slideIn: {
          from: { transform: "translateX(-20px)", opacity: "0" },
          to: { transform: "translateX(0)", opacity: "1" },
        },
        fadeIn: {
          from: { opacity: "0" },
          to: { opacity: "1" },
        },
        ringSpin: {
          to: { transform: "rotate(360deg)" },
        },
      },
      backdropBlur: {
        xs: "2px",
      },
    },
  },
  plugins: [],
};
