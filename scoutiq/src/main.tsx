import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { AuthProvider } from "./context/AuthContext";
import { CloudSyncProvider } from "./cloud";
import { SaveProvider } from "./save";
import { SquadProvider } from "./squad/SquadContext";
import "./i18n/i18n";
import { ThemeProvider } from "./theme/ThemeProvider";
import "./styles/global.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <AuthProvider>
        <SaveProvider>
          <CloudSyncProvider>
            <SquadProvider>
              <App />
            </SquadProvider>
          </CloudSyncProvider>
        </SaveProvider>
      </AuthProvider>
    </ThemeProvider>
  </React.StrictMode>,
);
