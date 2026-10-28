import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { api, toAppError, type ConnectionView, type TokenExpiry } from "../../../api";
import { SETTINGS_KEYS } from "../../../queries/settings";
import { fail } from "../../../state/store";
import { Group } from "../controls";
import { TextField } from "../fields";

function expiryText(expiry: TokenExpiry): string {
  switch (expiry.state) {
    case "soon":
      return expiry.days === 0 ? "Expires today" : `Expires in ${expiry.days} ${expiry.days === 1 ? "day" : "days"}`;
    case "expired":
      return "Expired; create a new token";
    case "fine":
      return "Taskboard warns 14 days ahead.";
  }
}

/** Connection → Token: when the API token runs out; saves on its own, with the rest of the saved connection. */
export function TokenGroup({ view }: { view: ConnectionView }) {
  const client = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const { connection } = view;

  const saveExpiry = (date: string) => {
    const input = { siteUrl: connection.siteUrl, email: connection.email, tokenKind: connection.tokenKind, tokenExpires: date || null, cloudId: null };
    api.connectionSave(input, null).then(
      (next) => {
        setError(null);
        client.setQueryData(SETTINGS_KEYS.connection, next);
      },
      (e: unknown) => {
        const err = toAppError(e);
        if (err.field) setError(err.field.message);
        else fail(err, "Could not save the expiry date");
      },
    );
  };

  return (
    <Group title="Token" anchor="token">
      <TextField
        label="Expires on"
        type="date"
        description={expiryText(view.expiry)}
        value={connection.tokenExpires ?? ""}
        error={error}
        onCommit={saveExpiry}
      />
    </Group>
  );
}
