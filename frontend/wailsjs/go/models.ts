export namespace main {
	
	export class MetricsSettings {
	    enableObservatory: boolean;
	    observatoryListen: string;
	    observatoryToken: string;
	
	    static createFrom(source: any = {}) {
	        return new MetricsSettings(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.enableObservatory = source["enableObservatory"];
	        this.observatoryListen = source["observatoryListen"];
	        this.observatoryToken = source["observatoryToken"];
	    }
	}
	export class RegionRoutingSettings {
	    proxyCountries: string[];
	    directCountries: string[];
	    blockCountries: string[];
	
	    static createFrom(source: any = {}) {
	        return new RegionRoutingSettings(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.proxyCountries = source["proxyCountries"];
	        this.directCountries = source["directCountries"];
	        this.blockCountries = source["blockCountries"];
	    }
	}
	export class DNSUpstream {
	    tag: string;
	    type: string;
	    address: string;
	    detour?: string;
	    strategy?: string;
	
	    static createFrom(source: any = {}) {
	        return new DNSUpstream(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.tag = source["tag"];
	        this.type = source["type"];
	        this.address = source["address"];
	        this.detour = source["detour"];
	        this.strategy = source["strategy"];
	    }
	}
	export class DNSSettings {
	    strategy: string;
	    servers: DNSUpstream[];
	
	    static createFrom(source: any = {}) {
	        return new DNSSettings(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.strategy = source["strategy"];
	        this.servers = this.convertValues(source["servers"], DNSUpstream);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	export class SplitTunnelSettings {
	    bypassDomains: string[];
	    bypassIPs: string[];
	    bypassProcesses: string[];
	    proxyDomains: string[];
	    proxyIPs: string[];
	    proxyProcesses: string[];
	    blockDomains: string[];
	    blockIPs: string[];
	    blockProcesses: string[];
	
	    static createFrom(source: any = {}) {
	        return new SplitTunnelSettings(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.bypassDomains = source["bypassDomains"];
	        this.bypassIPs = source["bypassIPs"];
	        this.bypassProcesses = source["bypassProcesses"];
	        this.proxyDomains = source["proxyDomains"];
	        this.proxyIPs = source["proxyIPs"];
	        this.proxyProcesses = source["proxyProcesses"];
	        this.blockDomains = source["blockDomains"];
	        this.blockIPs = source["blockIPs"];
	        this.blockProcesses = source["blockProcesses"];
	    }
	}
	export class ConnectRequest {
	    VLESSURI: string;
	    Mode: string;
	    SplitTunnel?: SplitTunnelSettings;
	    DNS?: DNSSettings;
	    RegionRouting?: RegionRoutingSettings;
	    Metrics?: MetricsSettings;
	
	    static createFrom(source: any = {}) {
	        return new ConnectRequest(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.VLESSURI = source["VLESSURI"];
	        this.Mode = source["Mode"];
	        this.SplitTunnel = this.convertValues(source["SplitTunnel"], SplitTunnelSettings);
	        this.DNS = this.convertValues(source["DNS"], DNSSettings);
	        this.RegionRouting = this.convertValues(source["RegionRouting"], RegionRoutingSettings);
	        this.Metrics = this.convertValues(source["Metrics"], MetricsSettings);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	
	
	
	

}

