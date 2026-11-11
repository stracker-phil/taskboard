/** A byte count in megabytes with one decimal ("38.2 MB"). */
export const megabytes = (bytes: number) => `${(bytes / 1_000_000).toFixed(1)} MB`;
