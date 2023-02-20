"use strict";(self.webpackChunkdocs=self.webpackChunkdocs||[]).push([[789],{3905:(e,n,t)=>{t.d(n,{Zo:()=>p,kt:()=>m});var i=t(7294);function r(e,n,t){return n in e?Object.defineProperty(e,n,{value:t,enumerable:!0,configurable:!0,writable:!0}):e[n]=t,e}function o(e,n){var t=Object.keys(e);if(Object.getOwnPropertySymbols){var i=Object.getOwnPropertySymbols(e);n&&(i=i.filter((function(n){return Object.getOwnPropertyDescriptor(e,n).enumerable}))),t.push.apply(t,i)}return t}function a(e){for(var n=1;n<arguments.length;n++){var t=null!=arguments[n]?arguments[n]:{};n%2?o(Object(t),!0).forEach((function(n){r(e,n,t[n])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(t)):o(Object(t)).forEach((function(n){Object.defineProperty(e,n,Object.getOwnPropertyDescriptor(t,n))}))}return e}function l(e,n){if(null==e)return{};var t,i,r=function(e,n){if(null==e)return{};var t,i,r={},o=Object.keys(e);for(i=0;i<o.length;i++)t=o[i],n.indexOf(t)>=0||(r[t]=e[t]);return r}(e,n);if(Object.getOwnPropertySymbols){var o=Object.getOwnPropertySymbols(e);for(i=0;i<o.length;i++)t=o[i],n.indexOf(t)>=0||Object.prototype.propertyIsEnumerable.call(e,t)&&(r[t]=e[t])}return r}var c=i.createContext({}),u=function(e){var n=i.useContext(c),t=n;return e&&(t="function"==typeof e?e(n):a(a({},n),e)),t},p=function(e){var n=u(e.components);return i.createElement(c.Provider,{value:n},e.children)},s="mdxType",f={inlineCode:"code",wrapper:function(e){var n=e.children;return i.createElement(i.Fragment,{},n)}},d=i.forwardRef((function(e,n){var t=e.components,r=e.mdxType,o=e.originalType,c=e.parentName,p=l(e,["components","mdxType","originalType","parentName"]),s=u(t),d=r,m=s["".concat(c,".").concat(d)]||s[d]||f[d]||o;return t?i.createElement(m,a(a({ref:n},p),{},{components:t})):i.createElement(m,a({ref:n},p))}));function m(e,n){var t=arguments,r=n&&n.mdxType;if("string"==typeof e||r){var o=t.length,a=new Array(o);a[0]=d;var l={};for(var c in n)hasOwnProperty.call(n,c)&&(l[c]=n[c]);l.originalType=e,l[s]="string"==typeof e?e:r,a[1]=l;for(var u=2;u<o;u++)a[u]=t[u];return i.createElement.apply(null,a)}return i.createElement.apply(null,t)}d.displayName="MDXCreateElement"},9964:(e,n,t)=>{t.r(n),t.d(n,{assets:()=>c,contentTitle:()=>a,default:()=>f,frontMatter:()=>o,metadata:()=>l,toc:()=>u});var i=t(7462),r=(t(7294),t(3905));const o={title:"Configuration Reference",sidebar_position:1,toc_max_heading_level:4},a=void 0,l={unversionedId:"config/reference",id:"config/reference",title:"Configuration Reference",description:"Reference for the configuration files.",source:"@site/tutorial/config/reference.mdx",sourceDirName:"config",slug:"/config/reference",permalink:"/tutorial/config/reference",draft:!1,editUrl:"https://github.com/dnbln/upsilon/blob/trunk/docs/tutorial/config/reference.mdx",tags:[],version:"current",lastUpdatedBy:"Dinu Blanovschi",lastUpdatedAt:1676900760,formattedLastUpdatedAt:"Feb 20, 2023",sidebarPosition:1,frontMatter:{title:"Configuration Reference",sidebar_position:1,toc_max_heading_level:4},sidebar:"contributor_guideSidebar",previous:{title:"Configuration",permalink:"/tutorial/category/configuration-1"}},c={},u=[{value:"Configuration files",id:"configuration-files",level:2},{value:"Selecting a profile",id:"selecting-a-profile",level:2},{value:"Configuration file reference",id:"configuration-file-reference",level:2}],p={toc:u},s="wrapper";function f(e){let{components:n,...t}=e;return(0,r.kt)(s,(0,i.Z)({},p,t,{components:n,mdxType:"MDXLayout"}),(0,r.kt)("p",null,"Reference for the configuration files."),(0,r.kt)("h2",{id:"configuration-files"},"Configuration files"),(0,r.kt)("p",null,"The configuration files are written in YAML."),(0,r.kt)("p",null,"They are loaded in the following order:"),(0,r.kt)("ul",null,(0,r.kt)("li",{parentName:"ul"},"from ",(0,r.kt)("inlineCode",{parentName:"li"},"UPSILON_CONFIG")," (default: ",(0,r.kt)("inlineCode",{parentName:"li"},"upsilon.dev.yaml")," in development builds,\n",(0,r.kt)("inlineCode",{parentName:"li"},"upsilon.yaml")," in production builds)"),(0,r.kt)("li",{parentName:"ul"},"from ",(0,r.kt)("inlineCode",{parentName:"li"},"UPSILON_ROCKET_CONFIG")," (default: ",(0,r.kt)("inlineCode",{parentName:"li"},"upsilon-rocket.yaml"),")")),(0,r.kt)("h2",{id:"selecting-a-profile"},"Selecting a profile"),(0,r.kt)("p",null,"The ",(0,r.kt)("inlineCode",{parentName:"p"},"UPSILON_PROFILE")," environment variable (default: ",(0,r.kt)("inlineCode",{parentName:"p"},"dev")," in development\nbuilds, ",(0,r.kt)("inlineCode",{parentName:"p"},"release")," in production builds), can be used to select which of those\nconfiguration files to use."),(0,r.kt)("p",null,"If the ",(0,r.kt)("inlineCode",{parentName:"p"},"UPSILON_PROFILE")," environment variable is set to ",(0,r.kt)("inlineCode",{parentName:"p"},"dev"),", the configuration\nfile that will be used is ",(0,r.kt)("inlineCode",{parentName:"p"},"upsilon.dev.yaml"),", unless the ",(0,r.kt)("inlineCode",{parentName:"p"},"UPSILON_CONFIG"),"\nenvironment variable is also set, in which case that configuration will be\nused instead."),(0,r.kt)("p",null,"If it is set to ",(0,r.kt)("inlineCode",{parentName:"p"},"release"),", the configuration file that will be used is\n",(0,r.kt)("inlineCode",{parentName:"p"},"upsilon.yaml"),", unless the ",(0,r.kt)("inlineCode",{parentName:"p"},"UPSILON_CONFIG")," environment variable is also\nset, in which case that configuration will be used instead."),(0,r.kt)("h2",{id:"configuration-file-reference"},"Configuration file reference"),(0,r.kt)("p",null,"Files from ",(0,r.kt)("inlineCode",{parentName:"p"},"UPSILON_CONFIG"),", (",(0,r.kt)("inlineCode",{parentName:"p"},"upsilon.dev.yaml")," or ",(0,r.kt)("inlineCode",{parentName:"p"},"upsilon.yaml")," by default)\nshould have the structure given by the JSON schema available here:"),(0,r.kt)("p",null,(0,r.kt)("a",{parentName:"p",href:"https://github.com/dnbln/upsilon/blob/trunk/schemas/upsilon-config.schema.json"},"https://github.com/dnbln/upsilon/blob/trunk/schemas/upsilon-config.schema.json")))}f.isMDXComponent=!0}}]);