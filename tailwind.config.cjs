/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.{js,ts,jsx,tsx}",
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}"
  ],
  theme: {
    extend: {
      colors: {
        primary: '#2563eb',
        secondary: '#10b981',
        dark: '#111827',
        light: '#f3f4f6'
      },
      borderRadius: {
        lg: '0.5rem',
        md: '0.375rem',
        sm: '0.25rem',
        xs: '0.125rem',
        xl: '0.75rem',
        xxl: '1rem',
        xxxl: '1.25rem'
      },
      boxShadow: {
        sm: '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
        md: '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)',
        lg: '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -1px rgba(0, 0, 0, 0.05)',
        xl: '0 14px 28px -12px rgba(0, 0, 0, 0.2), 0 10px 10px -6px rgba(0, 0, 0, 0.1)',
        '2xl': '0 18px 33px -6px rgba(0, 0, 0, 0.2), 0 6px 10px -6px rgba(0, 0, 0, 0.15)',
        '3xl': '0 19px 38px -8px rgba(0, 0, 0, 0.25), 0 12px 16px -6px rgba(0, 0, 0, 0.25)',
        '4xl': '0 23px 40px -9px rgba(0, 0, 0, 0.3), 0 14px 28px -10px rgba(0, 0, 0, 0.3)',
        '5xl': '0 24px 45px -10px rgba(0, 0, 0, 0.35), 0 19px 30px -14px rgba(0, 0, 0, 0.35)',
        '6xl': '0 25px 50px -12px rgba(0, 0, 0, 0.35), 0 19px 25px -10px rgba(0, 0, 0, 0.35)',
        '7xl': '0 27px 58px -12px rgba(0, 0, 0, 0.35), 0 19px 25px -12px rgba(0, 0, 0, 0.35)',
        '8xl': '0 33px 74px -15px rgba(0, 0, 0, 0.35), 0 22px 38px -15px rgba(0, 0, 0, 0.35)',
        '9xl': '0 37px 86px -16px rgba(0, 0, 0, 0.35), 0 26px 38px -17px rgba(0, 0, 0, 0.35)',
        '10xl': '0 40px 90px -16px rgba(0, 0, 0, 0.35), 0 26px 38px -17px rgba(0, 0, 0, 0.35)',
      }
    }
  },
  plugins: [],
}