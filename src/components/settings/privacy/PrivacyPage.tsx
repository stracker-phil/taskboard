import { ReadsGroup } from "../jira/ChangesGroup";
import { StorageGroup } from "./StorageGroup";

/** Settings → Privacy & Data: where the Jira data lives and what Taskboard reads. */
export function PrivacyPage() {
  return (
    <>
      <StorageGroup />
      <ReadsGroup />
    </>
  );
}
