// Web NFC helpers. Only works on Android Chrome over HTTPS.

export type NfcSupport = 'supported' | 'unsupported';

export function nfcSupported(): NfcSupport {
	if (typeof window === 'undefined') return 'unsupported';
	return 'NDEFReader' in window ? 'supported' : 'unsupported';
}

type NDEFReadingEventLike = { serialNumber?: string };

interface NDEFReaderLike {
	scan(options: { signal: AbortSignal }): Promise<void>;
	onreadingerror: (() => void) | null;
	onreading: ((event: NDEFReadingEventLike) => void) | null;
}

export class NfcScanner {
	private reader: NDEFReaderLike | null = null;
	private controller: AbortController | null = null;
	private onRead: (serial: string) => void;
	private onError: (err: Error) => void;

	constructor(onRead: (serial: string) => void, onError: (err: Error) => void) {
		this.onRead = onRead;
		this.onError = onError;
	}

	async start() {
		if (!nfcSupported()) {
			this.onError(new Error('Web NFC not supported on this device/browser.'));
			return;
		}
		try {
			const NDEFReaderCtor = (window as unknown as { NDEFReader: new () => NDEFReaderLike })
				.NDEFReader;
			this.reader = new NDEFReaderCtor();
			this.controller = new AbortController();
			await this.reader.scan({ signal: this.controller.signal });
			this.reader.onreadingerror = () => this.onError(new Error('Cannot read this card.'));
			this.reader.onreading = (event: NDEFReadingEventLike) => {
				const serial = (event.serialNumber ?? '').toLowerCase().replace(/[^a-f0-9:]/g, '');
				if (serial) this.onRead(serial);
			};
		} catch (e: unknown) {
			this.onError(e instanceof Error ? e : new Error(String(e)));
		}
	}

	stop() {
		this.controller?.abort();
		this.controller = null;
		this.reader = null;
	}
}
