(function() {var implementors = {};
implementors["hal"] = [{"text":"impl Copy for MemoryType","synthetic":false,"types":[]},{"text":"impl Copy for MemoryMapEntry","synthetic":false,"types":[]},{"text":"impl Copy for LoadedImage","synthetic":false,"types":[]},{"text":"impl Copy for Segment","synthetic":false,"types":[]},{"text":"impl Copy for VideoModeInfo","synthetic":false,"types":[]},{"text":"impl Copy for PixelFormat","synthetic":false,"types":[]},{"text":"impl&lt;S:&nbsp;Copy&gt; Copy for Frame&lt;S&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;S: FrameSize,&nbsp;</span>","synthetic":false,"types":[]},{"text":"impl&lt;S:&nbsp;Copy + FrameSize&gt; Copy for Page&lt;S&gt;","synthetic":false,"types":[]},{"text":"impl Copy for Flags","synthetic":false,"types":[]},{"text":"impl Copy for PhysicalAddress","synthetic":false,"types":[]},{"text":"impl Copy for VirtualAddress","synthetic":false,"types":[]},{"text":"impl Copy for Size4KiB","synthetic":false,"types":[]},{"text":"impl Copy for Size2MiB","synthetic":false,"types":[]},{"text":"impl Copy for Size1GiB","synthetic":false,"types":[]},{"text":"impl Copy for PciAddress","synthetic":false,"types":[]}];
implementors["kernel"] = [{"text":"impl Copy for HoleInfo","synthetic":false,"types":[]},{"text":"impl Copy for KernelObjectId","synthetic":false,"types":[]}];
implementors["libpebble"] = [{"text":"impl Copy for Capability","synthetic":false,"types":[]},{"text":"impl Copy for GetFramebufferError","synthetic":false,"types":[]},{"text":"impl Copy for PixelFormat","synthetic":false,"types":[]},{"text":"impl Copy for FramebufferInfo","synthetic":false,"types":[]},{"text":"impl Copy for EarlyLogError","synthetic":false,"types":[]},{"text":"impl Copy for CreateMemoryObjectError","synthetic":false,"types":[]},{"text":"impl Copy for MapMemoryObjectError","synthetic":false,"types":[]},{"text":"impl Copy for SendMessageError","synthetic":false,"types":[]},{"text":"impl Copy for RegisterServiceError","synthetic":false,"types":[]},{"text":"impl Copy for SubscribeToServiceError","synthetic":false,"types":[]},{"text":"impl Copy for Handle","synthetic":false,"types":[]}];
implementors["log"] = [{"text":"impl Copy for Level","synthetic":false,"types":[]},{"text":"impl Copy for LevelFilter","synthetic":false,"types":[]}];
implementors["num_complex"] = [{"text":"impl&lt;T:&nbsp;Copy&gt; Copy for Complex&lt;T&gt;","synthetic":false,"types":[]}];
implementors["num_integer"] = [{"text":"impl&lt;A:&nbsp;Copy&gt; Copy for ExtendedGcd&lt;A&gt;","synthetic":false,"types":[]}];
implementors["num_rational"] = [{"text":"impl&lt;T:&nbsp;Copy&gt; Copy for Ratio&lt;T&gt;","synthetic":false,"types":[]},{"text":"impl Copy for ParseRatioError","synthetic":false,"types":[]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()