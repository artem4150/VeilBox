export namespace main {
	
	export class ConnectRequest {
	    VLESSURI: string;
	    Mode: string;
	
	    static createFrom(source: any = {}) {
	        return new ConnectRequest(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.VLESSURI = source["VLESSURI"];
	        this.Mode = source["Mode"];
	    }
	}

}

