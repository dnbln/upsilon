"use strict";(self.webpackChunkdocs=self.webpackChunkdocs||[]).push([[479],{3905:(e,t,n)=>{n.d(t,{Zo:()=>u,kt:()=>m});var i=n(7294);function r(e,t,n){return t in e?Object.defineProperty(e,t,{value:n,enumerable:!0,configurable:!0,writable:!0}):e[t]=n,e}function a(e,t){var n=Object.keys(e);if(Object.getOwnPropertySymbols){var i=Object.getOwnPropertySymbols(e);t&&(i=i.filter((function(t){return Object.getOwnPropertyDescriptor(e,t).enumerable}))),n.push.apply(n,i)}return n}function o(e){for(var t=1;t<arguments.length;t++){var n=null!=arguments[t]?arguments[t]:{};t%2?a(Object(n),!0).forEach((function(t){r(e,t,n[t])})):Object.getOwnPropertyDescriptors?Object.defineProperties(e,Object.getOwnPropertyDescriptors(n)):a(Object(n)).forEach((function(t){Object.defineProperty(e,t,Object.getOwnPropertyDescriptor(n,t))}))}return e}function s(e,t){if(null==e)return{};var n,i,r=function(e,t){if(null==e)return{};var n,i,r={},a=Object.keys(e);for(i=0;i<a.length;i++)n=a[i],t.indexOf(n)>=0||(r[n]=e[n]);return r}(e,t);if(Object.getOwnPropertySymbols){var a=Object.getOwnPropertySymbols(e);for(i=0;i<a.length;i++)n=a[i],t.indexOf(n)>=0||Object.prototype.propertyIsEnumerable.call(e,n)&&(r[n]=e[n])}return r}var l=i.createContext({}),p=function(e){var t=i.useContext(l),n=t;return e&&(n="function"==typeof e?e(t):o(o({},t),e)),n},u=function(e){var t=p(e.components);return i.createElement(l.Provider,{value:t},e.children)},c="mdxType",d={inlineCode:"code",wrapper:function(e){var t=e.children;return i.createElement(i.Fragment,{},t)}},h=i.forwardRef((function(e,t){var n=e.components,r=e.mdxType,a=e.originalType,l=e.parentName,u=s(e,["components","mdxType","originalType","parentName"]),c=p(n),h=r,m=c["".concat(l,".").concat(h)]||c[h]||d[h]||a;return n?i.createElement(m,o(o({ref:t},u),{},{components:n})):i.createElement(m,o({ref:t},u))}));function m(e,t){var n=arguments,r=t&&t.mdxType;if("string"==typeof e||r){var a=n.length,o=new Array(a);o[0]=h;var s={};for(var l in t)hasOwnProperty.call(t,l)&&(s[l]=t[l]);s.originalType=e,s[c]="string"==typeof e?e:r,o[1]=s;for(var p=2;p<a;p++)o[p]=n[p];return i.createElement.apply(null,o)}return i.createElement.apply(null,n)}h.displayName="MDXCreateElement"},9954:(e,t,n)=>{n.r(t),n.d(t,{assets:()=>l,contentTitle:()=>o,default:()=>d,frontMatter:()=>a,metadata:()=>s,toc:()=>p});var i=n(7462),r=(n(7294),n(3905));const a={sidebar_position:5},o="Testing",s={unversionedId:"architecture/testing",id:"architecture/testing",title:"Testing",description:"All integration tests go in dev/upsilon-testsuite, with",source:"@site/contributor-guide/architecture/testing.md",sourceDirName:"architecture",slug:"/architecture/testing",permalink:"/contributor-guide/architecture/testing",draft:!1,editUrl:"https://github.com/dnbln/upsilon/tree/trunk/docs/contributor-guide/architecture/testing.md",tags:[],version:"current",sidebarPosition:5,frontMatter:{sidebar_position:5},sidebar:"tutorialSidebar",previous:{title:"Interacting with git / libgit2",permalink:"/contributor-guide/architecture/git"}},l={},p=[{value:"Running the tests",id:"running-the-tests",level:2},{value:"Writing tests",id:"writing-tests",level:2},{value:"<code>#[git_daemon]</code>",id:"git_daemon",level:2},{value:"<code>#[git_ssh]</code>",id:"git_ssh",level:2},{value:"<code>#[offline]</code>",id:"offline",level:2},{value:"<code>#[test_attr(...)]</code>",id:"test_attr",level:2},{value:"Cleanup: internals",id:"cleanup-internals",level:2},{value:"Windows",id:"windows",level:3},{value:"Linux",id:"linux",level:3},{value:"Other platforms",id:"other-platforms",level:3}],u={toc:p},c="wrapper";function d(e){let{components:t,...n}=e;return(0,r.kt)(c,(0,i.Z)({},u,n,{components:t,mdxType:"MDXLayout"}),(0,r.kt)("h1",{id:"testing"},"Testing"),(0,r.kt)("p",null,"All integration tests go in ",(0,r.kt)("inlineCode",{parentName:"p"},"dev/upsilon-testsuite"),", with\n",(0,r.kt)("inlineCode",{parentName:"p"},"dev/upsilon-test-support")," providing some utilities for them."),(0,r.kt)("h2",{id:"running-the-tests"},"Running the tests"),(0,r.kt)("p",null,"To run the tests, you can use the ",(0,r.kt)("inlineCode",{parentName:"p"},"test")," xtask:"),(0,r.kt)("pre",null,(0,r.kt)("code",{parentName:"pre",className:"language-bash"},"cargo xtask test\n# Or for short\ncargo xt\n")),(0,r.kt)("h2",{id:"writing-tests"},"Writing tests"),(0,r.kt)("p",null,"Integration tests are annotated with ",(0,r.kt)("inlineCode",{parentName:"p"},"#[upsilon_test]"),", which handles most of\nthe setup, and provides a ",(0,r.kt)("inlineCode",{parentName:"p"},"TestCx")," to the test, which is used to interact with\nthe webserver."),(0,r.kt)("h2",{id:"git_daemon"},(0,r.kt)("inlineCode",{parentName:"h2"},"#[git_daemon]")),(0,r.kt)("p",null,"By default, the test server doesn't spawn a ",(0,r.kt)("inlineCode",{parentName:"p"},"git daemon"),", but can be configured\nto do so by annotating the ",(0,r.kt)("inlineCode",{parentName:"p"},"TestCx")," parameter with\n",(0,r.kt)("inlineCode",{parentName:"p"},"#[cfg_setup(upsilon_basic_config_with_git_daemon)]"),", or (recommended:)\nby annotating the whole test function with ",(0,r.kt)("inlineCode",{parentName:"p"},"#[git_daemon]"),"."),(0,r.kt)("h2",{id:"git_ssh"},(0,r.kt)("inlineCode",{parentName:"h2"},"#[git_ssh]")),(0,r.kt)("p",null,"Similarly, for using git over the ",(0,r.kt)("inlineCode",{parentName:"p"},"ssh://")," protocol, you can use\n",(0,r.kt)("inlineCode",{parentName:"p"},"#[cfg_setup(upsilon_basic_config_with_ssh)]")," or the (recommended) test\nattribute ",(0,r.kt)("inlineCode",{parentName:"p"},"#[git_ssh]"),"."),(0,r.kt)("p",null,"Note that this does not work on windows, as ",(0,r.kt)("inlineCode",{parentName:"p"},"git-shell")," is not available in\n",(0,r.kt)("inlineCode",{parentName:"p"},"git-for-windows"),", so ",(0,r.kt)("inlineCode",{parentName:"p"},"#[git_ssh]")," also implies\n",(0,r.kt)("inlineCode",{parentName:"p"},"#[test_attr(cfg_attr(windows, ignore))]"),"."),(0,r.kt)("h2",{id:"offline"},(0,r.kt)("inlineCode",{parentName:"h2"},"#[offline]")),(0,r.kt)("p",null,"This is an attribute that indicates that the test can work offline. It can\nreceive an optional argument, to control this behavior."),(0,r.kt)("pre",null,(0,r.kt)("code",{parentName:"pre",className:"language-rust"},"#[upsilon_test] // will run in offline mode (default)\nasync fn t1(cx: &mut TestCx) -> TestResult {\n    // ...\n    Ok(())\n}\n\n#[upsilon_test]\n#[offline] // will run in offline mode (explicit)\nasync fn t2(cx: &mut TestCx) -> TestResult {\n    // ...\n    Ok(())\n}\n\n#[upsilon_test]\n#[offline(run)] // will run in offline mode (explicit)\nasync fn t3(cx: &mut TestCx) -> TestResult {\n    // ...\n    Ok(())\n}\n\n#[upsilon_test]\n#[offline(ignore)] // will not run in offline mode (ignored)\nasync fn t4(cx: &mut TestCx) -> TestResult {\n    // ...\n    Ok(())\n}\n")),(0,r.kt)("p",null,"Tests that actually require to connect to some other server (over an internet\nconnection) should use ",(0,r.kt)("inlineCode",{parentName:"p"},"#[offline(ignore)]")," to make sure they are online when\nrunning."),(0,r.kt)("h2",{id:"test_attr"},(0,r.kt)("inlineCode",{parentName:"h2"},"#[test_attr(...)]")),(0,r.kt)("p",null,(0,r.kt)("inlineCode",{parentName:"p"},"test_attr")," is an attribute that can be used to annotate the actual test\nfunction. In practice, it can be used to ignore tests on certain platforms, if\nsomething the test requires is not available or not working properly on said\nplatforms, or to ignore tests that require a network connection when running in\noffline mode, although those specific cases are already handled by other\nattributes (see ",(0,r.kt)("inlineCode",{parentName:"p"},"#[git_ssh]"),", ",(0,r.kt)("inlineCode",{parentName:"p"},"#[offline]"),")."),(0,r.kt)("p",null,"At the very end, ",(0,r.kt)("inlineCode",{parentName:"p"},"#[upsilon_test]")," will also make sure to clean up the web\nserver and terminate any subprocesses it has spawned."),(0,r.kt)("p",null,"All tests return a ",(0,r.kt)("inlineCode",{parentName:"p"},"TestResult<()>"),", which is just\na ",(0,r.kt)("inlineCode",{parentName:"p"},"Result<(), anyhow::Error>"),". For the cleanup to work, tests should\nreturn ",(0,r.kt)("inlineCode",{parentName:"p"},"Err")," if they fail, rather than ",(0,r.kt)("inlineCode",{parentName:"p"},"panic!"),"."),(0,r.kt)("h2",{id:"cleanup-internals"},"Cleanup: internals"),(0,r.kt)("p",null,"The webserver is spawned in a separate process, under a\n",(0,r.kt)("inlineCode",{parentName:"p"},"upsilon-gracefully-shutdown-host")," process that acts as a middle-man, and\nlistens via ",(0,r.kt)("inlineCode",{parentName:"p"},"ctrlc")," to signals, or to the creation of a temporary file."),(0,r.kt)("h3",{id:"windows"},"Windows"),(0,r.kt)("p",null,"The ",(0,r.kt)("inlineCode",{parentName:"p"},"upsilon-gracefully-shutdown-host")," process is created with\nthe ",(0,r.kt)("inlineCode",{parentName:"p"},"CREATE_NEW_PROCESS_GROUP")," flag. When it receives an event\n(via ",(0,r.kt)("inlineCode",{parentName:"p"},"ctrlc"),"), or the temporary file is created, it generates a\n",(0,r.kt)("inlineCode",{parentName:"p"},"CTRL_BREAK_EVENT")," for itself, which is then propagated to all the child\nprocesses, thus shutting everything down."),(0,r.kt)("h3",{id:"linux"},"Linux"),(0,r.kt)("p",null,"The ",(0,r.kt)("inlineCode",{parentName:"p"},"upsilon-gracefully-shutdown-host")," process just waits for ",(0,r.kt)("inlineCode",{parentName:"p"},"SIGTERM"),",\n",(0,r.kt)("inlineCode",{parentName:"p"},"SIGINT")," or similar signals via ",(0,r.kt)("inlineCode",{parentName:"p"},"ctrlc"),", then walks the ",(0,r.kt)("inlineCode",{parentName:"p"},"procfs")," to find all the\ndescendants of the process, and sends ",(0,r.kt)("inlineCode",{parentName:"p"},"SIGTERM")," to each of them."),(0,r.kt)("h3",{id:"other-platforms"},"Other platforms"),(0,r.kt)("p",null,(0,r.kt)("inlineCode",{parentName:"p"},"procfs")," is only available on Linux, so separate implementations are needed for\nother platforms. The ",(0,r.kt)("inlineCode",{parentName:"p"},"upsilon-gracefully-shutdown-host")," binary just fails to\ncompile on other platforms right now, making it impossible to run tests on them."))}d.isMDXComponent=!0}}]);