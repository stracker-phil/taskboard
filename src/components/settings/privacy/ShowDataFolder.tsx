import { api } from "../../../api";
import { attempt } from "../../../state/requests";

/** Opens the folder with Taskboard's databases and settings in the Finder. */
export function ShowDataFolder() {
  return (
    <button type="button" className="btn" onClick={() => void attempt(api.runCommand("app:data"), "Could not open the data folder")}>
      Show Data Folder
    </button>
  );
}
