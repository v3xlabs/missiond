import v3xlabs from "eslint-plugin-v3xlabs";

export default [
  { ignores: ["dist/**", "src/api/schema.gen.ts"] },
  ...v3xlabs.configs.recommended,
  ...v3xlabs.configs.solid,
  ...v3xlabs.configs.tailwindcss,
  // A flat config has to default-export the array the plugin's own rule forbids everywhere else.
  { files: ["eslint.config.js"], rules: { "import/no-default-export": "off" } },
];
