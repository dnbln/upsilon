"use strict";(self.webpackChunkdocs=self.webpackChunkdocs||[]).push([[6080],{3905:(e,t,r)=>{r.d(t,{Zo:()=>u,kt:()=>m});var n=r(7294);function o(e,t,r){return t in e?Object.defineProperty(e,t,{value:r,enumerable:!0,configurable:!0,writable:!0}):e[t]=r,e}function a(e,t){var r=Object.keys(e);if(Object.getOwnPropertySymbols){var n=Object.getOwnPropertySymbols(e);t&&(n=n.filter((function(t){return Object.getOwnPropertyDescriptor(e,t).enumerable}))),r.push.apply(r,n)}return r}function i(e){for(var t=1;t<arguments.length;t++){var r=null!=arguments[t]?arguments[t]:{};t%2?a(Object(r),!0).forEach((function(t){o(e,t,r[t])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(r)):a(Object(r)).forEach((function(t){Object.defineProperty(e,t,Object.getOwnPropertyDescriptor(r,t))}))}return e}function c(e,t){if(null==e)return{};var r,n,o=function(e,t){if(null==e)return{};var r,n,o={},a=Object.keys(e);for(n=0;n<a.length;n++)r=a[n],t.indexOf(r)>=0||(o[r]=e[r]);return o}(e,t);if(Object.getOwnPropertySymbols){var a=Object.getOwnPropertySymbols(e);for(n=0;n<a.length;n++)r=a[n],t.indexOf(r)>=0||Object.prototype.propertyIsEnumerable.call(e,r)&&(o[r]=e[r])}return o}var s=n.createContext({}),l=function(e){var t=n.useContext(s),r=t;return e&&(r="function"==typeof e?e(t):i(i({},t),e)),r},u=function(e){var t=l(e.components);return n.createElement(s.Provider,{value:t},e.children)},p="mdxType",d={inlineCode:"code",wrapper:function(e){var t=e.children;return n.createElement(n.Fragment,{},t)}},f=n.forwardRef((function(e,t){var r=e.components,o=e.mdxType,a=e.originalType,s=e.parentName,u=c(e,["components","mdxType","originalType","parentName"]),p=l(r),f=o,m=p["".concat(s,".").concat(f)]||p[f]||d[f]||a;return r?n.createElement(m,i(i({ref:t},u),{},{components:r})):n.createElement(m,i({ref:t},u))}));function m(e,t){var r=arguments,o=t&&t.mdxType;if("string"==typeof e||o){var a=r.length,i=new Array(a);i[0]=f;var c={};for(var s in t)hasOwnProperty.call(t,s)&&(c[s]=t[s]);c.originalType=e,c[p]="string"==typeof e?e:o,i[1]=c;for(var l=2;l<a;l++)i[l]=r[l];return n.createElement.apply(null,i)}return n.createElement.apply(null,r)}f.displayName="MDXCreateElement"},4793:(e,t,r)=>{r.r(t),r.d(t,{assets:()=>s,contentTitle:()=>i,default:()=>d,frontMatter:()=>a,metadata:()=>c,toc:()=>l});var n=r(7462),o=(r(7294),r(3905));const a={sidebar_position:4,title:"Code style"},i=void 0,c={unversionedId:"code-style",id:"code-style",title:"Code style",description:"Rust",source:"@site/contributor-guide/code-style.mdx",sourceDirName:".",slug:"/code-style",permalink:"/contributor-guide/code-style",draft:!1,editUrl:"https://github.com/dnbln/upsilon/blob/trunk/docs/contributor-guide/code-style.mdx",tags:[],version:"current",lastUpdatedBy:"Dinu Blanovschi",lastUpdatedAt:1682104017,formattedLastUpdatedAt:"Apr 21, 2023",sidebarPosition:4,frontMatter:{sidebar_position:4,title:"Code style"},sidebar:"contributor_guideSidebar",previous:{title:"upsilon-debug-data-driver",permalink:"/contributor-guide/dev-crates/upsilon-debug-data-driver"}},s={},l=[{value:"Rust",id:"rust",level:2}],u={toc:l},p="wrapper";function d(e){let{components:t,...r}=e;return(0,o.kt)(p,(0,n.Z)({},u,r,{components:t,mdxType:"MDXLayout"}),(0,o.kt)("h2",{id:"rust"},"Rust"),(0,o.kt)("p",null,"For Rust code changes, make sure that the ",(0,o.kt)("inlineCode",{parentName:"p"},"lint")," and ",(0,o.kt)("inlineCode",{parentName:"p"},"fmt-check"),"\n",(0,o.kt)("a",{parentName:"p",href:"/contributor-guide/dev-crates/upsilon-xtask"},"xtasks")," pass.\nIf ",(0,o.kt)("inlineCode",{parentName:"p"},"fmt-check")," check fails, then the code can be reformatted with the ",(0,o.kt)("inlineCode",{parentName:"p"},"fmt"),"\nxtask."))}d.isMDXComponent=!0}}]);