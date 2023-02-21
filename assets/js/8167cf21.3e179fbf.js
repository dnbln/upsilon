"use strict";(self.webpackChunkdocs=self.webpackChunkdocs||[]).push([[3562],{3905:(e,t,r)=>{r.d(t,{Zo:()=>u,kt:()=>m});var n=r(7294);function a(e,t,r){return t in e?Object.defineProperty(e,t,{value:r,enumerable:!0,configurable:!0,writable:!0}):e[t]=r,e}function i(e,t){var r=Object.keys(e);if(Object.getOwnPropertySymbols){var n=Object.getOwnPropertySymbols(e);t&&(n=n.filter((function(t){return Object.getOwnPropertyDescriptor(e,t).enumerable}))),r.push.apply(r,n)}return r}function o(e){for(var t=1;t<arguments.length;t++){var r=null!=arguments[t]?arguments[t]:{};t%2?i(Object(r),!0).forEach((function(t){a(e,t,r[t])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(r)):i(Object(r)).forEach((function(t){Object.defineProperty(e,t,Object.getOwnPropertyDescriptor(r,t))}))}return e}function c(e,t){if(null==e)return{};var r,n,a=function(e,t){if(null==e)return{};var r,n,a={},i=Object.keys(e);for(n=0;n<i.length;n++)r=i[n],t.indexOf(r)>=0||(a[r]=e[r]);return a}(e,t);if(Object.getOwnPropertySymbols){var i=Object.getOwnPropertySymbols(e);for(n=0;n<i.length;n++)r=i[n],t.indexOf(r)>=0||Object.prototype.propertyIsEnumerable.call(e,r)&&(a[r]=e[r])}return a}var l=n.createContext({}),d=function(e){var t=n.useContext(l),r=t;return e&&(r="function"==typeof e?e(t):o(o({},t),e)),r},u=function(e){var t=d(e.components);return n.createElement(l.Provider,{value:t},e.children)},s="mdxType",p={inlineCode:"code",wrapper:function(e){var t=e.children;return n.createElement(n.Fragment,{},t)}},h=n.forwardRef((function(e,t){var r=e.components,a=e.mdxType,i=e.originalType,l=e.parentName,u=c(e,["components","mdxType","originalType","parentName"]),s=d(r),h=a,m=s["".concat(l,".").concat(h)]||s[h]||p[h]||i;return r?n.createElement(m,o(o({ref:t},u),{},{components:r})):n.createElement(m,o({ref:t},u))}));function m(e,t){var r=arguments,a=t&&t.mdxType;if("string"==typeof e||a){var i=r.length,o=new Array(i);o[0]=h;var c={};for(var l in t)hasOwnProperty.call(t,l)&&(c[l]=t[l]);c.originalType=e,c[s]="string"==typeof e?e:a,o[1]=c;for(var d=2;d<i;d++)o[d]=r[d];return n.createElement.apply(null,o)}return n.createElement.apply(null,r)}h.displayName="MDXCreateElement"},6128:(e,t,r)=>{r.r(t),r.d(t,{assets:()=>l,contentTitle:()=>o,default:()=>p,frontMatter:()=>i,metadata:()=>c,toc:()=>d});var n=r(7462),a=(r(7294),r(3905));r(3530);const i={title:"Data storage",sidebar_position:2},o=void 0,c={unversionedId:"architecture/data",id:"architecture/data",title:"Data storage",description:"upsilon-data",source:"@site/contributor-guide/architecture/data.mdx",sourceDirName:"architecture",slug:"/architecture/data",permalink:"/contributor-guide/architecture/data",draft:!1,editUrl:"https://github.com/dnbln/upsilon/blob/trunk/docs/contributor-guide/architecture/data.mdx",tags:[],version:"current",lastUpdatedBy:"Dinu Blanovschi",lastUpdatedAt:1677012913,formattedLastUpdatedAt:"Feb 21, 2023",sidebarPosition:2,frontMatter:{title:"Data storage",sidebar_position:2},sidebar:"contributor_guideSidebar",previous:{title:"Introduction",permalink:"/contributor-guide/architecture/introduction"},next:{title:"API",permalink:"/contributor-guide/architecture/api"}},l={},d=[{value:"<code>upsilon-data</code>",id:"upsilon-data",level:2},{value:"<code>upsilon-data-cache-inmemory</code>",id:"upsilon-data-cache-inmemory",level:2},{value:"<code>upsilon-data-inmemory</code>",id:"upsilon-data-inmemory",level:2}],u={toc:d},s="wrapper";function p(e){let{components:t,...r}=e;return(0,a.kt)(s,(0,n.Z)({},u,r,{components:t,mdxType:"MDXLayout"}),(0,a.kt)("h2",{id:"upsilon-data"},(0,a.kt)("inlineCode",{parentName:"h2"},"upsilon-data")),(0,a.kt)("p",null,"This crate provides the ",(0,a.kt)("inlineCode",{parentName:"p"},"DataClient"),' trait, which is implemented for a few\ndifferent "data backends", but also for the cache.'),(0,a.kt)("p",null,"It also provides the ",(0,a.kt)("inlineCode",{parentName:"p"},"DataClientMasterHolder")," struct, which is ",(0,a.kt)("inlineCode",{parentName:"p"},".manage()"),"d by\nRocket, and is used to get a ",(0,a.kt)("inlineCode",{parentName:"p"},"DataQueryMaster"),' to use in the handlers, which is\nbasically a nice wrapper over the "raw" interface provided by ',(0,a.kt)("inlineCode",{parentName:"p"},"QueryImpl"),", the\nimplementors of which do the actual work."),(0,a.kt)("h2",{id:"upsilon-data-cache-inmemory"},(0,a.kt)("inlineCode",{parentName:"h2"},"upsilon-data-cache-inmemory")),(0,a.kt)("p",null,'The cache is a special data client, which caches the results of the other data\nclients, and if the result of a query is already in the cache, it will return\nthe cached result, instead of querying the data backend it is "wrapping".'),(0,a.kt)("h2",{id:"upsilon-data-inmemory"},(0,a.kt)("inlineCode",{parentName:"h2"},"upsilon-data-inmemory")),(0,a.kt)("p",null,"This is a data backend, which stores all the data in memory, and is used for\ntesting mostly."))}p.isMDXComponent=!0},3530:(e,t,r)=>{r.d(t,{Z:()=>i});var n=r(7294),a=r(9031);function i(e){let{crate:t,kind:r,gitref:i}=e,o=`${r}/${t}`;return n.createElement(a.Z,{gitref:i,kind:"tree",path:o,children:[n.createElement("code",null,t)]})}},9031:(e,t,r)=>{r.d(t,{Z:()=>i});var n=r(7294);const a="trunk";function i(e){let{gitref:t,path:r,kind:i,children:o}=e,c=`https://github.com/dnbln/upsilon/${i}/${t||a}/${r}`;return n.createElement("a",{href:c},o)}}}]);