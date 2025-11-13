/// React frontend generation for real-time dashboard.

use super::utils::generate_entity_display_config;
use crate::codegen::EntityDef;
use std::path::Path;
use std::error::Error;

/// Generate React frontend code
pub fn generate_frontend(
    entities: &[EntityDef],
    output_dir: &Path,
    _config_dir: &str,
) -> Result<(), Box<dyn Error>> {
    // Create directory structure
    std::fs::create_dir_all(output_dir.join("src/generated"))?;
    std::fs::create_dir_all(output_dir.join("src/components"))?;
    std::fs::create_dir_all(output_dir.join("src/hooks"))?;

    // Generate package.json
    generate_package_json(output_dir)?;

    // Generate TypeScript config
    generate_tsconfig(output_dir)?;

    // Generate Vite config
    generate_vite_config(output_dir)?;

    // Generate index.html
    generate_index_html(output_dir)?;

    // Generate entities TypeScript config
    generate_entities_ts(entities, output_dir)?;

    // Generate main App component
    generate_app_tsx(output_dir)?;

    // Generate main.tsx entry point
    generate_main_tsx(output_dir)?;

    // Generate Tailwind CSS config
    generate_tailwind_config(output_dir)?;

    // Generate CSS file
    generate_index_css(output_dir)?;

    // Generate WebSocket hook
    generate_use_realtime_data_hook(entities, output_dir)?;

    // Generate EntityCard component
    generate_entity_card_component(output_dir)?;

    // Generate Dashboard component
    generate_dashboard_component(output_dir)?;

    Ok(())
}

/// Generate package.json
fn generate_package_json(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let package_file = output_dir.join("package.json");
    let mut output = std::fs::File::create(&package_file)?;

    use std::io::Write;

    writeln!(output, "{{")?;
    writeln!(output, "  \"name\": \"dashboard-frontend\",")?;
    writeln!(output, "  \"version\": \"0.1.0\",")?;
    writeln!(output, "  \"type\": \"module\",")?;
    writeln!(output, "  \"scripts\": {{")?;
    writeln!(output, "    \"dev\": \"vite\",")?;
    writeln!(output, "    \"build\": \"tsc && vite build\",")?;
    writeln!(output, "    \"preview\": \"vite preview\"")?;
    writeln!(output, "  }},")?;
    writeln!(output, "  \"dependencies\": {{")?;
    writeln!(output, "    \"react\": \"^18.2.0\",")?;
    writeln!(output, "    \"react-dom\": \"^18.2.0\"")?;
    writeln!(output, "  }},")?;
    writeln!(output, "  \"devDependencies\": {{")?;
    writeln!(output, "    \"@types/react\": \"^18.2.43\",")?;
    writeln!(output, "    \"@types/react-dom\": \"^18.2.17\",")?;
    writeln!(output, "    \"@vitejs/plugin-react\": \"^4.2.1\",")?;
    writeln!(output, "    \"typescript\": \"^5.3.3\",")?;
    writeln!(output, "    \"vite\": \"^5.0.11\",")?;
    writeln!(output, "    \"tailwindcss\": \"^3.4.0\",")?;
    writeln!(output, "    \"postcss\": \"^8.4.33\",")?;
    writeln!(output, "    \"autoprefixer\": \"^10.4.16\"")?;
    writeln!(output, "  }}")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate tsconfig.json
fn generate_tsconfig(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let tsconfig_file = output_dir.join("tsconfig.json");
    let mut output = std::fs::File::create(&tsconfig_file)?;

    use std::io::Write;

    writeln!(output, "{{")?;
    writeln!(output, "  \"compilerOptions\": {{")?;
    writeln!(output, "    \"target\": \"ES2020\",")?;
    writeln!(output, "    \"useDefineForClassFields\": true,")?;
    writeln!(output, "    \"lib\": [\"ES2020\", \"DOM\", \"DOM.Iterable\"],")?;
    writeln!(output, "    \"module\": \"ESNext\",")?;
    writeln!(output, "    \"skipLibCheck\": true,")?;
    writeln!(output, "    \"moduleResolution\": \"bundler\",")?;
    writeln!(output, "    \"allowImportingTsExtensions\": true,")?;
    writeln!(output, "    \"resolveJsonModule\": true,")?;
    writeln!(output, "    \"isolatedModules\": true,")?;
    writeln!(output, "    \"noEmit\": true,")?;
    writeln!(output, "    \"jsx\": \"react-jsx\",")?;
    writeln!(output, "    \"strict\": true,")?;
    writeln!(output, "    \"noUnusedLocals\": true,")?;
    writeln!(output, "    \"noUnusedParameters\": true,")?;
    writeln!(output, "    \"noFallthroughCasesInSwitch\": true")?;
    writeln!(output, "  }},")?;
    writeln!(output, "  \"include\": [\"src\"]")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate vite.config.ts
fn generate_vite_config(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let vite_file = output_dir.join("vite.config.ts");
    let mut output = std::fs::File::create(&vite_file)?;

    use std::io::Write;

    writeln!(output, "import {{ defineConfig }} from 'vite'")?;
    writeln!(output, "import react from '@vitejs/plugin-react'\n")?;

    writeln!(output, "export default defineConfig({{")?;
    writeln!(output, "  plugins: [react()],")?;
    writeln!(output, "  server: {{")?;
    writeln!(output, "    host: '0.0.0.0',")?;
    writeln!(output, "    port: 5173,")?;
    writeln!(output, "  }},")?;
    writeln!(output, "}})")?;

    Ok(())
}

/// Generate index.html
fn generate_index_html(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let index_file = output_dir.join("index.html");
    let mut output = std::fs::File::create(&index_file)?;

    use std::io::Write;

    writeln!(output, "<!DOCTYPE html>")?;
    writeln!(output, "<html lang=\"en\">")?;
    writeln!(output, "  <head>")?;
    writeln!(output, "    <meta charset=\"UTF-8\" />")?;
    writeln!(output, "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />")?;
    writeln!(output, "    <title>Real-Time Dashboard</title>")?;
    writeln!(output, "  </head>")?;
    writeln!(output, "  <body>")?;
    writeln!(output, "    <div id=\"root\"></div>")?;
    writeln!(output, "    <script type=\"module\" src=\"/src/main.tsx\"></script>")?;
    writeln!(output, "  </body>")?;
    writeln!(output, "</html>")?;

    Ok(())
}

/// Generate src/generated/entities.ts
fn generate_entities_ts(entities: &[EntityDef], output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let entities_file = output_dir.join("src/generated/entities.ts");
    let mut output = std::fs::File::create(&entities_file)?;

    use std::io::Write;

    writeln!(output, "// Auto-generated entity configuration")?;
    writeln!(output, "// This file is regenerated when entities change\n")?;

    // Type definitions
    writeln!(output, "export interface Entity {{")?;
    writeln!(output, "  name: string;")?;
    writeln!(output, "  table: string;")?;
    writeln!(output, "  color: string;")?;
    writeln!(output, "  icon: string;")?;
    writeln!(output, "  fields: string[];")?;
    writeln!(output, "  maxRecords: number;")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "export const ENTITIES: Entity[] = [")?;

    for entity in entities {
        if !entity.is_persistent() || entity.is_abstract {
            continue;
        }

        if entity.source_type.to_lowercase() == "reference" {
            continue;
        }

        let display_config = generate_entity_display_config(entity);

        writeln!(output, "  {{")?;
        writeln!(output, "    name: \"{}\",", display_config.name)?;
        writeln!(output, "    table: \"{}\",", display_config.table)?;
        writeln!(output, "    color: \"{}\",", display_config.color)?;
        writeln!(output, "    icon: \"{}\",", display_config.icon)?;
        write!(output, "    fields: [")?;
        for (i, field) in display_config.display_fields.iter().enumerate() {
            if i > 0 {
                write!(output, ", ")?;
            }
            write!(output, "\"{}\"", field)?;
        }
        writeln!(output, "],")?;
        writeln!(output, "    maxRecords: {},", display_config.max_records)?;
        writeln!(output, "  }},")?;
    }

    writeln!(output, "];")?;

    println!("cargo:rerun-if-changed={}", entities_file.display());
    Ok(())
}

/// Generate src/App.tsx
fn generate_app_tsx(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let app_file = output_dir.join("src/App.tsx");
    let mut output = std::fs::File::create(&app_file)?;

    use std::io::Write;

    writeln!(output, "// Auto-generated App component")?;
    writeln!(output, "import {{ Dashboard }} from './components/Dashboard';\n")?;

    writeln!(output, "function App() {{")?;
    writeln!(output, "  return <Dashboard />;")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "export default App;")?;

    Ok(())
}

/// Generate src/main.tsx
fn generate_main_tsx(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let main_file = output_dir.join("src/main.tsx");
    let mut output = std::fs::File::create(&main_file)?;

    use std::io::Write;

    writeln!(output, "import React from 'react';")?;
    writeln!(output, "import ReactDOM from 'react-dom/client';")?;
    writeln!(output, "import App from './App';")?;
    writeln!(output, "import './index.css';\n")?;

    writeln!(output, "ReactDOM.createRoot(document.getElementById('root')!).render(")?;
    writeln!(output, "  <React.StrictMode>")?;
    writeln!(output, "    <App />")?;
    writeln!(output, "  </React.StrictMode>")?;
    writeln!(output, ");")?;

    Ok(())
}

/// Generate Tailwind config
fn generate_tailwind_config(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let tailwind_file = output_dir.join("tailwind.config.js");
    let mut output = std::fs::File::create(&tailwind_file)?;

    use std::io::Write;

    writeln!(output, "/** @type {{import('tailwindcss').Config}} */")?;
    writeln!(output, "export default {{")?;
    writeln!(output, "  content: [")?;
    writeln!(output, "    \"./index.html\",")?;
    writeln!(output, "    \"./src/**/*.{{js,ts,jsx,tsx}}\",")?;
    writeln!(output, "  ],")?;
    writeln!(output, "  theme: {{")?;
    writeln!(output, "    extend: {{}},")?;
    writeln!(output, "  }},")?;
    writeln!(output, "  plugins: [],")?;
    writeln!(output, "}};")?;

    Ok(())
}

/// Generate src/index.css
fn generate_index_css(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let css_file = output_dir.join("src/index.css");
    let mut output = std::fs::File::create(&css_file)?;

    use std::io::Write;

    writeln!(output, "@tailwind base;")?;
    writeln!(output, "@tailwind components;")?;
    writeln!(output, "@tailwind utilities;")?;

    Ok(())
}
/// Generate src/hooks/useRealtimeData.ts
fn generate_use_realtime_data_hook(_entities: &[EntityDef], output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let hook_file = output_dir.join("src/hooks/useRealtimeData.ts");
    let mut output = std::fs::File::create(&hook_file)?;

    use std::io::Write;

    writeln!(output, "// Auto-generated WebSocket hook for real-time data")?;
    writeln!(output, "import {{ useState, useEffect, useRef }} from 'react';")?;
    writeln!(output, "import {{ ENTITIES }} from '../generated/entities';\n")?;

    writeln!(output, "export interface EntityRecord {{")?;
    writeln!(output, "  entity: string;")?;
    writeln!(output, "  data: Record<string, any>;")?;
    writeln!(output, "  timestamp: string;")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "export interface RealtimeData {{")?;
    writeln!(output, "  records: Map<string, EntityRecord[]>;")?;
    writeln!(output, "  connected: boolean;")?;
    writeln!(output, "  error: string | null;")?;
    writeln!(output, "}}\n")?;

    // Generate dynamic WebSocket URL that works in K8s and local dev
    writeln!(output, "// Construct WebSocket URL from current location (works in K8s and local dev)")?;
    writeln!(output, "const getWebSocketUrl = () => {{")?;
    writeln!(output, "  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';")?;
    writeln!(output, "  const host = window.location.hostname;")?;
    writeln!(output, "  // Use NodePort 32390 for K8s, port 8080 for localhost")?;
    writeln!(output, "  const port = host === 'localhost' ? '8080' : '32390';")?;
    writeln!(output, "  return `${{protocol}}//${{host}}:${{port}}/ws`;")?;
    writeln!(output, "}};")?;
    writeln!(output, "const BACKEND_URL = getWebSocketUrl();\n")?;

    writeln!(output, "export function useRealtimeData(): RealtimeData {{")?;
    writeln!(output, "  const [records, setRecords] = useState<Map<string, EntityRecord[]>>(new Map());")?;
    writeln!(output, "  const [connected, setConnected] = useState(false);")?;
    writeln!(output, "  const [error, setError] = useState<string | null>(null);")?;
    writeln!(output, "  const wsRef = useRef<WebSocket | null>(null);\n")?;

    writeln!(output, "  useEffect(() => {{")?;
    writeln!(output, "    // Initialize records map with empty arrays for each entity")?;
    writeln!(output, "    const initialRecords = new Map<string, EntityRecord[]>();")?;
    writeln!(output, "    ENTITIES.forEach(entity => {{")?;
    writeln!(output, "      initialRecords.set(entity.name, []);")?;
    writeln!(output, "    }});")?;
    writeln!(output, "    setRecords(initialRecords);\n")?;

    writeln!(output, "    // Connect to WebSocket")?;
    writeln!(output, "    const ws = new WebSocket(BACKEND_URL);\n")?;

    writeln!(output, "    ws.onopen = () => {{")?;
    writeln!(output, "      console.log('WebSocket connected');")?;
    writeln!(output, "      setConnected(true);")?;
    writeln!(output, "      setError(null);")?;
    writeln!(output, "    }};\n")?;

    writeln!(output, "    ws.onmessage = (event) => {{")?;
    writeln!(output, "      try {{")?;
    writeln!(output, "        const message = JSON.parse(event.data);")?;
    writeln!(output, "        const {{ entity, data, timestamp }} = message;\n")?;

    writeln!(output, "        // Add new record to the appropriate entity")?;
    writeln!(output, "        setRecords(prev => {{")?;
    writeln!(output, "          const newRecords = new Map(prev);")?;
    writeln!(output, "          const entityRecords = newRecords.get(entity) || [];")?;
    writeln!(output, "          const entityConfig = ENTITIES.find(e => e.name === entity);\n")?;

    writeln!(output, "          // Add new record at the beginning (most recent first)")?;
    writeln!(output, "          const updatedRecords = [{{ entity, data, timestamp }}, ...entityRecords];\n")?;

    writeln!(output, "          // Cap at maxRecords")?;
    writeln!(output, "          const maxRecords = entityConfig?.maxRecords || 500;")?;
    writeln!(output, "          const cappedRecords = updatedRecords.slice(0, maxRecords);\n")?;

    writeln!(output, "          newRecords.set(entity, cappedRecords);")?;
    writeln!(output, "          return newRecords;")?;
    writeln!(output, "        }});")?;
    writeln!(output, "      }} catch (err) {{")?;
    writeln!(output, "        console.error('Failed to parse WebSocket message:', err);")?;
    writeln!(output, "      }}")?;
    writeln!(output, "    }};\n")?;

    writeln!(output, "    ws.onerror = (event) => {{")?;
    writeln!(output, "      console.error('WebSocket error:', event);")?;
    writeln!(output, "      setError('WebSocket connection error');")?;
    writeln!(output, "      setConnected(false);")?;
    writeln!(output, "    }};\n")?;

    writeln!(output, "    ws.onclose = () => {{")?;
    writeln!(output, "      console.log('WebSocket disconnected');")?;
    writeln!(output, "      setConnected(false);")?;
    writeln!(output, "    }};\n")?;

    writeln!(output, "    wsRef.current = ws;\n")?;

    writeln!(output, "    // Cleanup on unmount")?;
    writeln!(output, "    return () => {{")?;
    writeln!(output, "      ws.close();")?;
    writeln!(output, "    }};")?;
    writeln!(output, "  }}, []);\n")?;

    writeln!(output, "  return {{ records, connected, error }};")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate src/components/EntityCard.tsx
fn generate_entity_card_component(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let component_file = output_dir.join("src/components/EntityCard.tsx");
    let mut output = std::fs::File::create(&component_file)?;

    use std::io::Write;

    writeln!(output, "// Auto-generated EntityCard component")?;
    writeln!(output, "import {{ Entity }} from '../generated/entities';")?;
    writeln!(output, "import {{ EntityRecord }} from '../hooks/useRealtimeData';\n")?;

    writeln!(output, "interface EntityCardProps {{")?;
    writeln!(output, "  entity: Entity;")?;
    writeln!(output, "  records: EntityRecord[];")?;
    writeln!(output, "}}\n")?;

    writeln!(output, "export function EntityCard({{ entity, records }}: EntityCardProps) {{")?;
    writeln!(output, "  return (")?;
    writeln!(output, "    <div className=\"bg-white rounded-lg shadow-lg p-6 mb-6\">")?;
    writeln!(output, "      <div className=\"flex items-center gap-3 mb-4\">")?;
    writeln!(output, "        <span className=\"text-3xl\">{{entity.icon}}</span>")?;
    writeln!(output, "        <div>")?;
    writeln!(output, "          <h2 className=\"text-2xl font-bold\">{{entity.name}}</h2>")?;
    writeln!(output, "          <p className=\"text-sm text-gray-500\">Table: {{entity.table}}</p>")?;
    writeln!(output, "        </div>")?;
    writeln!(output, "        <div className=\"ml-auto\">")?;
    writeln!(output, "          <span className=\"text-sm font-semibold text-gray-700\">")?;
    writeln!(output, "            {{records.length}} records")?;
    writeln!(output, "          </span>")?;
    writeln!(output, "        </div>")?;
    writeln!(output, "      </div>\n")?;

    writeln!(output, "      <div className=\"overflow-x-auto\">")?;
    writeln!(output, "        <table className=\"min-w-full divide-y divide-gray-200\">")?;
    writeln!(output, "          <thead className=\"bg-gray-50\">")?;
    writeln!(output, "            <tr>")?;
    writeln!(output, "              <th className=\"px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase\">")?;
    writeln!(output, "                Time")?;
    writeln!(output, "              </th>")?;
    writeln!(output, "              {{entity.fields.map(field => (")?;
    writeln!(output, "                <th")?;
    writeln!(output, "                  key={{field}}")?;
    writeln!(output, "                  className=\"px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase\"")?;
    writeln!(output, "                >")?;
    writeln!(output, "                  {{field}}")?;
    writeln!(output, "                </th>")?;
    writeln!(output, "              ))}}")?;
    writeln!(output, "            </tr>")?;
    writeln!(output, "          </thead>")?;
    writeln!(output, "          <tbody className=\"bg-white divide-y divide-gray-200\">")?;
    writeln!(output, "            {{records.slice(0, 10).map((record, idx) => (")?;
    writeln!(output, "              <tr")?;
    writeln!(output, "                key={{idx}}")?;
    writeln!(output, "                className=\"hover:bg-gray-50 transition-colors\"")?;
    writeln!(output, "                style={{{{")?;
    writeln!(output, "                  borderLeft: `4px solid ${{entity.color}}`,")?;
    writeln!(output, "                }}}}")?;
    writeln!(output, "              >")?;
    writeln!(output, "                <td className=\"px-4 py-3 whitespace-nowrap text-sm text-gray-500\">")?;
    writeln!(output, "                  {{new Date(record.timestamp).toLocaleTimeString()}}")?;
    writeln!(output, "                </td>")?;
    writeln!(output, "                {{entity.fields.map(field => (")?;
    writeln!(output, "                  <td")?;
    writeln!(output, "                    key={{field}}")?;
    writeln!(output, "                    className=\"px-4 py-3 whitespace-nowrap text-sm text-gray-900\"")?;
    writeln!(output, "                  >")?;
    writeln!(output, "                    {{String(record.data[field] ?? '-')}}")?;
    writeln!(output, "                  </td>")?;
    writeln!(output, "                ))}}")?;
    writeln!(output, "              </tr>")?;
    writeln!(output, "            ))}}")?;
    writeln!(output, "          </tbody>")?;
    writeln!(output, "        </table>")?;
    writeln!(output, "        {{records.length === 0 && (")?;
    writeln!(output, "          <div className=\"text-center py-8 text-gray-500\">")?;
    writeln!(output, "            No records yet. Waiting for data...")?;
    writeln!(output, "          </div>")?;
    writeln!(output, "        )}}")?;
    writeln!(output, "      </div>")?;
    writeln!(output, "    </div>")?;
    writeln!(output, "  );")?;
    writeln!(output, "}}")?;

    Ok(())
}

/// Generate src/components/Dashboard.tsx
fn generate_dashboard_component(output_dir: &Path) -> Result<(), Box<dyn Error>> {
    let component_file = output_dir.join("src/components/Dashboard.tsx");
    let mut output = std::fs::File::create(&component_file)?;

    use std::io::Write;

    writeln!(output, "// Auto-generated Dashboard component")?;
    writeln!(output, "import {{ ENTITIES }} from '../generated/entities';")?;
    writeln!(output, "import {{ useRealtimeData }} from '../hooks/useRealtimeData';")?;
    writeln!(output, "import {{ EntityCard }} from './EntityCard';\n")?;

    writeln!(output, "export function Dashboard() {{")?;
    writeln!(output, "  const {{ records, connected, error }} = useRealtimeData();\n")?;

    writeln!(output, "  return (")?;
    writeln!(output, "    <div className=\"min-h-screen bg-gray-100\">")?;
    writeln!(output, "      <header className=\"bg-white shadow-md\">")?;
    writeln!(output, "        <div className=\"max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8\">")?;
    writeln!(output, "          <div className=\"flex items-center justify-between\">")?;
    writeln!(output, "            <h1 className=\"text-3xl font-bold text-gray-900\">")?;
    writeln!(output, "              Real-Time Dashboard")?;
    writeln!(output, "            </h1>")?;
    writeln!(output, "            <div className=\"flex items-center gap-2\">")?;
    writeln!(output, "              <div")?;
    writeln!(output, "                className={{`w-3 h-3 rounded-full ${{connected ? 'bg-green-500' : 'bg-red-500'}}`}}")?;
    writeln!(output, "              />")?;
    writeln!(output, "              <span className=\"text-sm font-medium text-gray-700\">")?;
    writeln!(output, "                {{connected ? 'Connected' : 'Disconnected'}}")?;
    writeln!(output, "              </span>")?;
    writeln!(output, "            </div>")?;
    writeln!(output, "          </div>")?;
    writeln!(output, "        </div>")?;
    writeln!(output, "      </header>\n")?;

    writeln!(output, "      <main className=\"max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8\">")?;
    writeln!(output, "        {{error && (")?;
    writeln!(output, "          <div className=\"bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-6\">")?;
    writeln!(output, "            Error: {{error}}")?;
    writeln!(output, "          </div>")?;
    writeln!(output, "        )}}\n")?;

    writeln!(output, "        {{ENTITIES.map(entity => (")?;
    writeln!(output, "          <EntityCard")?;
    writeln!(output, "            key={{entity.name}}")?;
    writeln!(output, "            entity={{entity}}")?;
    writeln!(output, "            records={{records.get(entity.name) || []}}")?;
    writeln!(output, "          />")?;
    writeln!(output, "        ))}}")?;
    writeln!(output, "      </main>")?;
    writeln!(output, "    </div>")?;
    writeln!(output, "  );")?;
    writeln!(output, "}}")?;

    Ok(())
}
