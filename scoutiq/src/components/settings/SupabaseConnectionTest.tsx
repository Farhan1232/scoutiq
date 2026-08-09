import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  testSupabaseConnection,
  type ConnectionCheck,
  type ConnectionTestResult,
} from "../../services/supabaseConnectionTest";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import "./SupabaseConnectionTest.css";

function toneFor(status: ConnectionCheck["status"]) {
  if (status === "ok") return "accent" as const;
  if (status === "warn") return "warning" as const;
  if (status === "error") return "danger" as const;
  return "neutral" as const;
}

export function SupabaseConnectionTest() {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<ConnectionTestResult | null>(null);

  const runTest = async () => {
    setBusy(true);
    try {
      setResult(await testSupabaseConnection());
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    void runTest();
  }, []);

  return (
    <div className="connection-test">
      <div className="connection-test__head">
        <div>
          <p className="connection-test__title">{t("cloud.connection.title")}</p>
          <p className="connection-test__subtitle">
            {t("cloud.connection.subtitle")}
          </p>
        </div>
        <Button
          variant="secondary"
          size="sm"
          disabled={busy}
          onClick={() => {
            void runTest();
          }}
        >
          {busy ? t("cloud.connection.testing") : t("cloud.connection.test")}
        </Button>
      </div>

      {result ? (
        <>
          <div className="connection-test__summary">
            <Badge tone={result.ok ? "accent" : "danger"}>
              {result.ok
                ? t("cloud.connection.summaryOk")
                : t("cloud.connection.summaryError")}
            </Badge>
            <span className="connection-test__time">
              {t("cloud.connection.checkedAt", {
                time: new Date(result.checkedAt).toLocaleString(),
              })}
            </span>
          </div>

          <ul className="connection-test__checks">
            {result.checks.map((check) => (
              <li key={check.id} className="connection-test__check">
                <span>{t(`cloud.connection.labels.${check.id}`)}</span>
                <div className="connection-test__check-meta">
                  <Badge tone={toneFor(check.status)} size="sm">
                    {t(`cloud.connection.status.${check.status}`)}
                  </Badge>
                  <strong>
                    {t(check.detailKey, check.detailParams)}
                  </strong>
                </div>
              </li>
            ))}
          </ul>
        </>
      ) : (
        <p className="connection-test__pending">{t("cloud.connection.testing")}</p>
      )}
    </div>
  );
}
