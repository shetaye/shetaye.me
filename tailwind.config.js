/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs"],
  darkMode: 'media', // Use prefers-color-scheme
  theme: {
    extend: {
      fontFamily: {
        sans: ['"Inter Tight"', 'system-ui', 'sans-serif'],
        mono: ['"Source Code Pro"', 'Consolas', 'Monaco', 'monospace'],
      },
      colors: {
        border: {
          DEFAULT: '#ddd',
          dark: '#333',
        },
      },
    },
  },
  plugins: [],
}
